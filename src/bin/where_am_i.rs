use anyhow::Result;
use lazy_static::lazy_static;
use prometheus::{register_gauge, Gauge};
use std::{
    convert::TryFrom,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, Level};
use tracing_subscriber::filter::EnvFilter;
use where_am_i::{
    configuration::{Configuration, GpsdConfig},
    devices::Devices,
    gpsd::Server,
    prometheus::Exporter,
};

lazy_static! {
    static ref START_TIME: Gauge = register_gauge!(
        "process_start_time_seconds",
        "Start time of the process since unix epoch in seconds."
    )
    .unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = match Configuration::load_from_next_arg() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load configuration file: {:?}", e);
            std::process::exit(1);
        }
    };

    start_tracing(&config);
    start_prometheus(&config).await?;
    let devices = start_devices(&config).await?;
    start_gpsd(&config, devices).await
}

async fn start_devices(config: &Configuration) -> Result<Devices> {
    Devices::start(&config.gps).await
}

async fn start_gpsd(config: &Configuration, devices: Devices) -> Result<()> {
    let gpsd_config = match &config.gpsd {
        Some(c) => c.clone(),
        None => GpsdConfig::default(),
    };

    let server = Server::new(gpsd_config, devices);
    server.run().await
}

async fn start_prometheus(config: &Configuration) -> Result<()> {
    if let Some(prometheus) = &config.prometheus {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH);

        if let Ok(duration) = start_time {
            START_TIME.set(duration.as_secs_f64());
        }

        for bind_address in &prometheus.bind_addresses {
            Exporter::new(bind_address.to_string())?.start().await
        }
    }

    Ok(())
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
