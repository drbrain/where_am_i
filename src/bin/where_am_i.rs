use std::convert::TryFrom;

use tokio::runtime;

use tokio_serial::SerialPortSettings;

use tracing::error;
use tracing::info;
use tracing::Level;

use tracing_subscriber::filter::EnvFilter;

use where_am_i::configuration::Configuration;
use where_am_i::configuration::GpsConfig;
use where_am_i::gps::GPS;
use where_am_i::gpsd::Server;
use where_am_i::nmea;
use where_am_i::nmea::UBX_OUTPUT_MESSAGES;
use where_am_i::pps;
use where_am_i::shm::NtpShm;

fn main() {
    let mut runtime = runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .on_thread_start(|| {
            eprintln!("thread started");
        })
        .on_thread_stop(|| {
            eprintln!("thread stopped");
        })
        .core_threads(2)
        .build()
        .unwrap();

    runtime.block_on(run());
}

async fn run() {
    let config = load_config();

    start_tracing(&config);

    let mut server = Server::new(2947);

    for gps_config in config.gps.iter() {
        start_gps(gps_config, &mut server).await;
    }

    server.run().await.unwrap();
}

fn start_tracing(config: &Configuration) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    let filter = tracing::subscriber::with_default(subscriber, || {
        match EnvFilter::try_from(config.clone()) {
            Ok(f) => f,
            Err(e) => {
                match &config.log_filter {
                    Some(f) => error!("invalid log_filter \"{}\": {:?}", f, e),
                    None => unreachable!(),
                };

                std::process::exit(1);
            }
        }
    });

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");
}

fn load_config() -> Configuration {
    let file = match std::env::args().nth(1) {
        None => {
            error!("You must provide a configuration file");
            std::process::exit(1);
        }
        Some(f) => f,
    };

    match Configuration::load(file.clone()) {
        Ok(c) => {
            info!("Loaded configuration from {}", file);
            c
        }
        Err(e) => {
            error!("failed to load configuration file: {:?}", e);
            std::process::exit(1);
        }
    }
}

async fn start_gps(gps_config: &GpsConfig, server: &mut Server) {
    let name = gps_config.name.clone();
    let gps_name = gps_config.device.clone();
    let messages = gps_config.messages.clone().unwrap_or(vec![]);

    let serial_port_settings = match SerialPortSettings::try_from(gps_config.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    let mut device = nmea::Device::new(gps_name.clone(), serial_port_settings);

    if messages.is_empty() {
        for message in &UBX_OUTPUT_MESSAGES {
            device.message(message, true);
        }
    } else {
        for default in &UBX_OUTPUT_MESSAGES {
            let enabled = messages.contains(&default.to_string());

            device.message(&default.to_string(), enabled);
        }
    }

    let gps_tx = device.run().await;

    let mut gps = GPS::new(gps_name.clone(), gps_tx.clone());

    gps.read().await;

    server.add_gps(&gps);
    info!("registered GPS {}", name.clone());

    if let Some(ntp_unit) = gps_config.ntp_unit {
        NtpShm::relay(ntp_unit, gps.ntp_tx.subscribe()).await;
        info!("Sending GPS time from {} via NTP unit {}", name, ntp_unit);
    }

    match &gps_config.pps {
        Some(pps_config) => {
            let pps_name = pps_config.device.clone();

            let mut pps = pps::Device::new(pps_name.clone(), gps_name.clone());

            match pps.run().await {
                Ok(()) => (),
                Err(e) => {
                    error!("Error opening PPS device {}: {}", pps_name, e);
                    std::process::exit(1);
                }
            };

            server.add_pps(&pps, gps_name.clone());
            info!("registered PPS {} under {}", pps_name, gps_name);

            if let Some(ntp_unit) = pps_config.ntp_unit {
                NtpShm::relay(ntp_unit, pps.tx.subscribe()).await;
                info!(
                    "Sending PPS time from {} via NTP unit {}",
                    pps_name, ntp_unit
                );
            }

            Some(pps)
        }
        None => None,
    };
}
