use std::convert::TryFrom;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use tokio::runtime;

use tracing::error;
use tracing::Level;

use tracing_subscriber::filter::EnvFilter;

use where_am_i::configuration::Configuration;
use where_am_i::configuration::GpsdConfig;
use where_am_i::gpsd::Server;

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

    let gpsd_config = match &config.gpsd {
        Some(c) => c.clone(),
        None => GpsdConfig::default(),
    };

    let mut server = Server::new(gpsd_config, config.gps.clone());

    server.start_gps_devices().await;

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
