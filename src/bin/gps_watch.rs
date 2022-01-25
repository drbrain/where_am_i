use anyhow::Result;
use std::convert::TryFrom;
use tracing::error;
use tracing::info;
use tracing_subscriber::filter::EnvFilter;
use where_am_i::configuration::Configuration;
use where_am_i::gps::GPS;
use where_am_i::nmea::NMEA;

#[tokio::main]
async fn main() -> Result<()> {
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

    let mut gps = GPS::new(&device).await?;
    let mut rx = gps.subscribe_nmea();

    gps.read().await;

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

    Ok(())
}
