use std::net::ToSocketAddrs;

use byte_unit::Byte;
use clap::{Arg, ArgMatches, Command};
use crab_net::{manager, Parameters, payload::PayloadConfig};
use log::{info, warn, LevelFilter};
use mimalloc::MiMalloc;
use simple_logger::SimpleLogger;
use tokio::runtime::{Builder, Runtime};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let cli = build_cli();
    let rt = build_runtime(&cli);

    rt.block_on(async {
        manager(extract_parameters(cli)).await;
    });
}

fn build_cli() -> ArgMatches {
    Command::new("crab-net")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Simple stress test for servers")
        .arg(
            Arg::new("addr")
                .short('d')
                .long("destination")
                .help("Server address as IP:PORT")
                .required(true),
        )
        .arg(
            Arg::new("clients")
                .short('c')
                .long("connections")
                .help("Number of clients to simulate")
                .default_value("1")
                .value_parser(clap::value_parser!(usize))
                .required(false),
        )
        .arg(
            Arg::new("payload")
                .short('l')
                .long("payload")
                .help("Custom payload string to send (fallback if not using payload file)")
                .default_value("test")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("payload-file")
                .long("payload-file")
                .help("YAML file containing multiple payloads")
                .value_parser(clap::value_parser!(String))
                .required(false),
        )
        .arg(
            Arg::new("payload-index")
                .long("payload-index")
                .help("Use specific payload index from file")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("random-payload")
                .long("random-payload")
                .help("Randomly select payload from file")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("rate")
                .short('r')
                .long("rate")
                .help("Defined as packets/sec")
                .default_value("1")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("Starting source port for clients")
                .default_value("8000")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .help("Number of worker threads for the Tokio runtime [default: #CPU core]")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("timeout")
                .short('s')
                .long("timeout")
                .help("Timeout between consecutive connections spawn as ms")
                .default_value("50")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new("udp")
                .long("udp")
                .help("Send packets via UDP")
                .num_args(0)
                .default_missing_value("true")
                .default_value("false")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            Arg::new("tls")
                .long("tls")
                .help("Send data over TLS")
                .num_args(0)
                .default_missing_value("true")
                .default_value("false")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            Arg::new("ca")
                .long("ca")
                .help("PEM File to validate server credentials")
                .value_parser(clap::value_parser!(String)),
        )
        .get_matches()
}

fn build_runtime(cli: &ArgMatches) -> Runtime {
    let worker_threads = cli.get_one::<usize>("workers");
    let mut rt_builder = Builder::new_multi_thread();
    if let Some(workers) = worker_threads {
        if *workers > 0 {
            rt_builder.worker_threads(*workers);
        }
    } else {
        warn!("Workers threads must be > 0. Switching to #CPU Core");
    }

    rt_builder.enable_all().build().unwrap()
}

fn extract_parameters(matches: ArgMatches) -> Parameters {
    let server_addr = matches
        .get_one::<String>("addr")
        .unwrap()
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let rate = *matches.get_one("rate").unwrap();
    let connections = *matches.get_one("clients").unwrap();
    let payload_file = matches.get_one::<String>("payload-file");
    let payload_index = matches.get_one::<usize>("payload-index").copied();
    let _random_payload = matches.get_flag("random-payload");
    
    let payload_config = if let Some(file) = payload_file {
        Some(PayloadConfig::from_file(file).unwrap())
    } else {
        None
    };

    let fallback_payload = matches.get_one::<String>("payload").unwrap().to_string();
    let len = if let Some(config) = &payload_config {
        if let Some(idx) = payload_index {
            config.get_payload(Some(idx), false).unwrap().len()
        } else if _random_payload {
            // Use first payload for size estimation since actual payload will be random
            config.payloads[0].data.len()
        } else {
            // Use first payload for size estimation
            config.payloads[0].data.len()
        }
    } else {
        fallback_payload.len()
    };
    let start_port = *matches.get_one("port").unwrap();
    let sleep = *matches.get_one("timeout").unwrap();

    let bandwidth = Byte::from_u128((connections * rate * len * 8) as u128)
        .unwrap_or_default()
        .get_appropriate_unit(byte_unit::UnitType::Decimal)
        .to_string();
    let bandwidth = bandwidth[0..bandwidth.len() - 1].to_string();

    let use_udp = *matches.get_one("udp").unwrap();
    let use_tls = *matches.get_one("tls").unwrap();
    let ca_file = matches.get_one("ca").cloned();

    info!("Server address: {server_addr}, clients: {connections}, payload size: {len}, rate: {rate} pkt/s, sleep timeout:{sleep} ms, udp: {use_udp}, tls: {use_tls}");
    info!("Theoretical Packets rate: {} pkt/sec", connections * rate);
    info!("Theoretical Bandwidth: {bandwidth} bit/s");

    Parameters::new(
        server_addr,
        rate,
        connections,
        payload_config,
        fallback_payload,
        start_port,
        sleep,
        (use_udp, (use_tls, ca_file)),
    )
}
