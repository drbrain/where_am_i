use std::convert::TryFrom;

use tracing::error;
use tracing::info;

use tracing_subscriber::filter::EnvFilter;

use where_am_i::configuration::Configuration;
use where_am_i::gps::GPS;
use where_am_i::gps::UBX_OUTPUT_MESSAGES;
use where_am_i::nmea::Device;
use where_am_i::nmea::NMEA;

use tokio_serial::SerialPortSettings;

#[tokio::main]
async fn main() {
    let config = match Configuration::load_from_next_arg() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load configuration file: {:?}", e);
            std::process::exit(1);
        }
    };

    let filter = match EnvFilter::try_from(config.clone()) {
        Ok(f) => f,
        Err(e) => {
            match config.log_filter {
                Some(f) => error!("invalid log_filter \"{}\": {:?}", f, e),
                None => unreachable!(),
            };

            std::process::exit(1);
        }
    };

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    let device = config.gps[0].clone();

    let gps_name = device.clone().device;
    let messages = device.clone().messages.unwrap_or(vec![]);

    let serial_port_settings = match SerialPortSettings::try_from(device.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    let mut device = Device::new(gps_name.clone(), device.gps_type, serial_port_settings);

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

    let tx = device.run().await;

    let mut gps = GPS::new(gps_name, tx.clone());

    gps.read().await;

    let mut rx = tx.subscribe();

    while let Ok(nmea) = rx.recv().await {
        match nmea {
            NMEA::InvalidChecksum(cm) => error!(
                "checksum match, given {}, calculated {} on {}",
                cm.given, cm.calculated, cm.message
            ),
            NMEA::ParseError(e) => error!("parse error: {}", e),
            NMEA::ParseFailure(f) => error!("parse failure: {}", f),
            NMEA::Unsupported(n) => error!("unsupported: {}", n),
            n => info!("{:?}", n),
        }
    }
}
