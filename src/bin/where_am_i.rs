use std::convert::TryFrom;

use tokio::runtime;

use tokio_serial::SerialPortSettings;

use tracing::error;
use tracing::info;
use tracing::Level;

use where_am_i::configuration::Configuration;
use where_am_i::gps::GPS;
use where_am_i::gpsd::Server;
use where_am_i::nmea::Device;
use where_am_i::nmea::UBX_OUTPUT_MESSAGES;
use where_am_i::pps::PPS;
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
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    let file = match std::env::args().nth(1) {
        None => {
            error!("You must provide a configuration file");
            std::process::exit(1);
        }
        Some(f) => f,
    };

    let config = match Configuration::load(file) {
        Ok(c) => c,
        Err(e) => {
            error!("failed to load configuration file: {:?}", e);
            std::process::exit(1);
        }
    };

    let gps_config = config.gps[0].clone();

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

    let mut device = Device::new(gps_name.clone(), serial_port_settings);

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

    let gps_tx = match device.run().await {
        Ok(t) => t,
        Err(e) => {
            error!("failed to read from GPS: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut gps = GPS::new(gps_name, gps_tx.clone());

    gps.read().await;

    let pps = match gps_config.pps {
        Some(p) => {
            let device_name = p.device.clone();

            let mut pps = PPS::new(device_name.clone(), name);

            match pps.run().await {
                Ok(()) => (),
                Err(e) => {
                    error!("Error opening PPS device {}: {}", device_name, e);
                    std::process::exit(1);
                }
            };

            Some(pps)
        }
        None => None,
    };

    let mut ntp_shm = NtpShm::new(2);
    let mut server = Server::new(2947);

    ntp_shm.add_gps(gps.tx.clone());
    server.add_gps(gps);
    info!("registered GPS");

    if let Some(p) = pps {
        let pps_device_name = p.device_name.clone();
        ntp_shm.add_pps(p.tx.clone());
        server.add_pps(p, pps_device_name);
        info!("registered PPS");
    }

    ntp_shm.run().await;
    server.run().await.unwrap();
}
