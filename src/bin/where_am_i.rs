use anyhow::Result;
use std::convert::TryFrom;
use tracing::error;
use tracing::Level;
use tracing_subscriber::filter::EnvFilter;
use where_am_i::{
    configuration::{Configuration, GpsdConfig},
    devices::Devices,
    gpsd::Server,
};

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

    let gpsd_config = match &config.gpsd {
        Some(c) => c.clone(),
        None => GpsdConfig::default(),
    };

    let devices = Devices::start(&config.gps).await?;

    let server = Server::new(gpsd_config, devices);
    server.run().await
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
