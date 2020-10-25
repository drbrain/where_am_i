use std::convert::TryFrom;

use tokio::sync::broadcast;

use tracing::error;
use tracing::info;
use tracing::debug;
use tracing::Level;

use tracing_subscriber::filter::EnvFilter;

use where_am_i::configuration::Configuration;
use where_am_i::configuration::GpsConfig;
use where_am_i::shm::NtpShm;
use where_am_i::TSSender;

#[tokio::main]
async fn main() {
    let config = load_config();
    let (tx, mut rx) = broadcast::channel(5);

    for gps_config in config.gps.iter() {
        if gps_config.ntp_unit.is_some() {
            NtpShmWatch::new(gps_config.clone(), tx.clone()).run().await;
        }
    }

    while let Ok(ts) = rx.recv().await {
        info!(
            "device: {} refclock: {}.{} received: {}.{}",
            ts.device, ts.real_sec, ts.real_nsec, ts.clock_sec, ts.clock_nsec
        );
    }
}

fn load_config() -> Configuration {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();

    let (config, filter) = tracing::subscriber::with_default(subscriber, || {
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

        (config, filter)
    });

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    config
}

struct NtpShmWatch {
    device: String,
    ntp_unit: i32,
    tx: TSSender,
}

impl NtpShmWatch {
    pub fn new(config: GpsConfig, tx: TSSender) -> Self {
        let device = config.device.clone();
        let ntp_unit = config.ntp_unit.unwrap();

        NtpShmWatch {
            device,
            ntp_unit,
            tx,
        }
    }

    pub async fn run(&self) {
        let device = self.device.clone();
        let ntp_unit = self.ntp_unit;
        let tx = self.tx.clone();

        tokio::spawn(async move {
            NtpShm::watch(ntp_unit, device, tx).await;
        });

        debug!("Watching for NTP SHM messages from {} on unit {}", self.device.clone(), self.ntp_unit);
    }
}
