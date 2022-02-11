use anyhow::Result;
use chrono::Duration;
use chrono::NaiveDateTime;
use std::convert::TryFrom;
use tracing::{debug, error, info};
use tracing_subscriber::filter::EnvFilter;
use where_am_i::configuration::Configuration;
use where_am_i::configuration::GpsConfig;
use where_am_i::shm::NtpShm;

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config();
    let local = tokio::task::LocalSet::new();

    for gps_config in config.gps.iter() {
        if gps_config.ntp_unit.is_some() {
            let gps_config = gps_config.clone();
            local.spawn_local(async move {
                match NtpShmWatch::new(&gps_config).run().await {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Starting GPS {}: {}", gps_config.device, e);
                        std::process::exit(1);
                    }
                }
            });
        }
    }

    Ok(())
}

fn load_config() -> Configuration {
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

    config
}

struct NtpShmWatch {
    device: String,
    ntp_unit: i32,
}

impl NtpShmWatch {
    pub fn new(config: &GpsConfig) -> Self {
        let device = config.device.clone();
        let ntp_unit = config.ntp_unit.unwrap();

        NtpShmWatch { device, ntp_unit }
    }

    pub async fn run(&self) -> Result<()> {
        let zero = Duration::seconds(0);
        let device = self.device.clone();
        let ntp_unit = self.ntp_unit;

        let ntp_shm = NtpShm::new(ntp_unit)?;

        debug!(
            "Watching for NTP SHM messages from {} on unit {}",
            &self.device, self.ntp_unit
        );

        ntp_shm
            .watch(|ts| {
                let received_time =
                    NaiveDateTime::from_timestamp(ts.receive_sec as i64, ts.receive_nsec);
                let reference_time =
                    NaiveDateTime::from_timestamp(ts.clock_sec as i64, ts.clock_nsec);

                let offset = reference_time.signed_duration_since(received_time);

                let offset_text = if offset > zero {
                    format!("{} after ", offset)
                } else {
                    format!("{} before", offset * -1)
                };

                info!(
                    "{} tick {} received {} system at {}",
                    device, reference_time, offset_text, received_time
                );
            })
            .await;

        Ok(())
    }
}
