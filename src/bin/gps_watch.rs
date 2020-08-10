mod args;

use std::collections::HashMap;

use tracing::error;
use tracing::info;
use tracing::Level;

use where_am_i::gps::GPS;
use where_am_i::nmea::Device;
use where_am_i::nmea::NMEA;
use where_am_i::nmea::UBX_OUTPUT_MESSAGES;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    let (gps_name, serial_port_settings, messages) = args::gps_watch_args();

    let mut device = Device::new(gps_name.clone(), serial_port_settings);

    if messages.len() == 0 {
        for message in &UBX_OUTPUT_MESSAGES {
            device.message(message, true);
        }
    } else {
        for default in &UBX_OUTPUT_MESSAGES {
            let enabled = if messages.contains(&default.to_string()) {
                true
            } else {
                false
            };

            device.message(&default.to_string(), enabled);
        }
    }

    let tx = match device.run().await {
        Ok(t) => t,
        Err(e) => {
            error!("failed to read from GPS: {:?}", e);
            std::process::exit(1);
        }
    };

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
