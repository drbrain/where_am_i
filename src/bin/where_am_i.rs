use std::convert::TryFrom;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use tokio::runtime;

use tokio_serial::SerialPortBuilder;

use tracing::error;
use tracing::info;
use tracing::Level;

use tracing_subscriber::filter::EnvFilter;

use where_am_i::configuration::Configuration;
use where_am_i::configuration::GpsConfig;
use where_am_i::gps::GPS;
use where_am_i::gpsd::Server;
use where_am_i::nmea;
use where_am_i::pps;
use where_am_i::shm::NtpShm;

fn main() {
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("where_am_i-{}", id)
        })
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(run());
}

async fn run() {
    let config = match Configuration::load_from_next_arg() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load configuration file: {:?}", e);
            std::process::exit(1);
        }
    };

    start_tracing(&config);

    let mut server = match &config.gpsd {
        Some(c) => Some(Server::new(c, config.gps.clone())),
        None => {
            eprintln!("GPSD server not configured");

            None
        }
    };

    for gps_config in config.gps.iter() {
        start_gps(gps_config, &mut server).await;
    }

    if let Some(s) = server {
        s.run().await.unwrap();
    }
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

async fn start_gps(gps_config: &GpsConfig, server: &mut Option<Server>) {
    let name = gps_config.name.clone();
    let gps_name = gps_config.device.clone();
    let messages = gps_config.messages.clone().unwrap_or(vec![]);

    let serial_port_settings = match SerialPortBuilder::try_from(gps_config.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    let device = nmea::Device::new(
        gps_name.clone(),
        gps_config.gps_type.clone(),
        serial_port_settings,
        messages,
    );

    let gps_tx = device.run().await;

    let mut gps = GPS::new(gps_name.clone(), gps_tx.clone());

    gps.read().await;

    if let Some(s) = server {
        s.add_gps(&gps);
    }

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

            if let Some(s) = server {
                s.add_pps(&pps, gps_name.clone());
            }

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
