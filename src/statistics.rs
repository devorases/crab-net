use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use byte_unit::Byte;
use kanal::{bounded_async, AsyncReceiver, AsyncSender};
use log::info;
use tokio::{
    select, spawn,
    sync::mpsc::Sender as TokioSender,
    time::{interval_at, Instant},
};

pub type StatPacket = (usize, usize);

// Struct to track total packets sent across the application
#[derive(Clone)]
pub struct StatsTracker {
    pub total_packets: Arc<AtomicUsize>,
    pub tx: AsyncSender<StatPacket>,
}

impl StatsTracker {
    pub fn new(tx: AsyncSender<StatPacket>, total_packets: Arc<AtomicUsize>) -> Self {
        Self { tx, total_packets }
    }

    pub fn get_total_packets(&self) -> usize {
        self.total_packets.load(Ordering::Relaxed)
    }
}

pub fn stats_task(
    clients: usize, 
    max_packets: Option<usize>,
    quit_tx: Option<TokioSender<()>>,
) -> StatsTracker {
    // Define channel to send statistics update
    let (stats_tx, stats_rx) = bounded_async(clients);
    
    // Create atomic counter for total packets
    let total_packets = Arc::new(AtomicUsize::new(0));
    let total_packets_clone = total_packets.clone();
    
    spawn(async move {
        stats_loop(stats_rx, total_packets_clone, max_packets, quit_tx).await;
    });
    
    StatsTracker::new(stats_tx, total_packets)
}

async fn stats_loop(
    stats_rx: AsyncReceiver<StatPacket>,
    total_packets: Arc<AtomicUsize>,
    max_packets: Option<usize>,
    quit_tx: Option<TokioSender<()>>,
) {
    let timer_duration = 10.;
    let duration = Duration::from_secs(timer_duration as u64);
    let mut timer = interval_at(Instant::now() + duration, duration);

    let mut bytes_sent = 0.;
    let mut interval_packets_sent = 0;
    
    loop {
        select! {
            _ = timer.tick() => {
                bytes_sent *= 8.;
                let bandwidth = Byte::from_f64(bytes_sent / timer_duration)
                    .unwrap_or_default()
                    .get_appropriate_unit(byte_unit::UnitType::Decimal)
                    .to_string();
                let bandwidth = &bandwidth[0..bandwidth.len()-1];
                
                let total = total_packets.load(Ordering::Relaxed);
                info!("Sent {interval_packets_sent} packets --- Bandwidth {bandwidth}bit/s --- Total packets: {total}");
                
                bytes_sent = 0.;
                interval_packets_sent = 0;
            }
            stat = stats_rx.recv() => if let Ok((bytes, packets)) = stat {
                bytes_sent += bytes as f64;
                interval_packets_sent += packets;
                
                // Update total packets counter
                let new_total = total_packets.fetch_add(packets, Ordering::Relaxed) + packets;
                
                // Check if we've reached the maximum packets limit
                if let Some(max) = max_packets {
                    if new_total >= max && quit_tx.is_some() {
                        let _ = quit_tx.unwrap().send(()).await;
                        info!("Reached target of {max} packets. Total sent: {new_total}");
                        break;
                    }
                }
            }
        }
    }
}
