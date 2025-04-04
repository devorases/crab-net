use std::{fs, io::Error, net::SocketAddr, path::Path, time::Duration};
use crate::payload::PayloadConfig;

use derive_new::new;
use log::{error, info};
use openssl::ssl::{SslContext, SslMethod};
use sender::{sender_task_dtls, sender_task_tcp, sender_task_udp};
use statistics::stats_task;
use tokio::{
    io::AsyncWrite,
    net::{TcpSocket, TcpStream, UdpSocket},
    task::JoinSet,
    time::sleep,
};
use tokio_dtls_stream_sink::{Client, Session};
use tokio_native_tls::native_tls::{Certificate, TlsConnector};

mod sender;
mod statistics;
pub mod payload;

pub async fn manager(params: Parameters) -> usize {
    let (udp, (use_tls, ca_file)) = params.connection_type;
    if use_tls && ca_file.is_none() {
        error!("DTLS requires CA file to verify server credentials");
        return 0;
    }

    // Setup quit channel for auto-termination
    let (quit_tx, mut quit_rx) = tokio::sync::mpsc::channel::<()>(1);
    
    // Initialize stats tracker with max packets if specified
    let stats_tracker = stats_task(params.connections, params.max_packets, Some(quit_tx));
    
    let mut tasks = JoinSet::new();
    let mut start_port = params.start_port;

    for id in 0..params.connections {
        start_port += id;
        let fallback_payload = params.payload.as_bytes().to_vec();
        let payload_config = params.payload_config.clone();
        let stats_tx_cloned = stats_tracker.tx.clone();
        let ca_file = ca_file.clone();
        let sequential_payload = params.sequential_payload;
        let random_payload = params.random_payload;
        
        if use_tls {
            if udp {
                let session =
                    setup_dtls_session(start_port, params.server_addr, ca_file.unwrap()).await;
                tasks.spawn(async move {
                    sender_task_dtls(id, session, fallback_payload, params.rate, stats_tx_cloned).await
                });
            } else {
                let stream =
                    setup_tls_stream(start_port, params.server_addr, ca_file.unwrap()).await;
                tasks.spawn(async move {
                    sender_task_tcp(id, stream, payload_config, fallback_payload, params.rate, stats_tx_cloned, 
                                   sequential_payload, random_payload).await
                });
            }
        } else if udp {
            let socket = setup_udp_socket(params.server_addr, start_port).await;
            tasks.spawn(async move {
                sender_task_udp(id, socket, payload_config, fallback_payload, params.rate, stats_tx_cloned,
                               sequential_payload, random_payload).await
            });
        } else {
            let stream = setup_tcp_stream(params.server_addr, start_port).await;
            tasks.spawn(async move {
                sender_task_tcp(id, stream, payload_config, fallback_payload, params.rate, stats_tx_cloned,
                               sequential_payload, random_payload).await
            });
        }
        sleep(Duration::from_millis(params.sleep)).await;
    }
    
    // Wait for either quit signal or all tasks to complete
    tokio::select! {
        _ = quit_rx.recv() => {
            info!("Received quit signal, shutting down...");
            tasks.abort_all();
        }
        _ = async { while (tasks.join_next().await).is_some() {} } => {
            info!("All tasks completed");
        }
    }
    
    // Return the total number of packets sent
    stats_tracker.get_total_packets()
}

async fn setup_udp_socket(addr: SocketAddr, port: usize) -> UdpSocket {
    let socket = UdpSocket::bind("0.0.0.0:".to_owned() + &port.to_string())
        .await
        .unwrap();
    socket.connect(addr).await.unwrap();
    socket
}

async fn setup_tcp_stream(addr: SocketAddr, port: usize) -> Box<TcpStream> {
    let local_addr = ("0.0.0.0:".to_owned() + &port.to_string()).parse().unwrap();
    let socket = TcpSocket::new_v4().unwrap();
    socket.bind(local_addr).unwrap();
    Box::new(socket.connect(addr).await.unwrap())
}

async fn setup_dtls_session(port: usize, addr: SocketAddr, ca_file: String) -> DtlsSession {
    let mut ctx = SslContext::builder(SslMethod::dtls()).unwrap();
    ctx.set_ca_file(ca_file).unwrap();
    let socket = UdpSocket::bind("0.0.0.0:".to_owned() + &port.to_string())
        .await
        .unwrap();
    let client = Client::new(socket);
    let session = client.connect(addr, Some(ctx.build())).await.unwrap();
    DtlsSession::new(client, session)
}

async fn setup_tls_stream(
    port: usize,
    addr: SocketAddr,
    ca_file: String,
) -> Box<dyn AsyncWrite + Unpin + Send> {
    let pem = fs::read(Path::new(&ca_file)).unwrap();
    let cert = Certificate::from_pem(&pem).unwrap();
    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        .danger_accept_invalid_hostnames(true)
        .build()
        .unwrap();
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let tcp_stream = setup_tcp_stream(addr, port).await;
    Box::new(
        connector
            .connect(addr.ip().to_string().as_str(), tcp_stream)
            .await
            .unwrap(),
    )
}


#[derive(new)]
pub struct Parameters {
    server_addr: SocketAddr,
    rate: usize,
    connections: usize,
    payload_config: Option<PayloadConfig>,
    payload: String, // fallback when not using payload_config
    start_port: usize,
    sleep: u64,
    connection_type: (bool, (bool, Option<String>)),
    max_packets: Option<usize>, // Maximum number of packets to send before quitting
    sequential_payload: bool,   // Use sequential payloads from file
    random_payload: bool,       // Use random payloads from file
}

#[derive(new)]
pub struct DtlsSession {
    _client: Client,
    session: Session,
}

impl DtlsSession {
    pub async fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.session.write(buf).await
    }
}
