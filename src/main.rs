mod args;
mod gps;
mod pps;
mod gpsd_server;

#[macro_use] extern crate nix;

use gps::GPS;
use gpsd_server::GpsdServer;
use pps::PPS;

use serde_json::Value;

use tokio::runtime;
use tokio::sync::broadcast;

use tracing::Level;
use tracing::error;

use tracing_subscriber;

pub type JsonReceiver = broadcast::Receiver<Value>;
pub type JsonSender = broadcast::Sender<Value>;

fn main() {
    let mut runtime =
        runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .on_thread_start(|| { eprintln!("thread started");})
        .on_thread_stop(|| { eprintln!("thread stopped");})
        .core_threads(2)
        .build()
        .unwrap();

    runtime.block_on(run());
}

async fn run() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("no global subscriber has been set");

    let (gps_name, serial_port_settings, pps_name) = args::parse();

    let gps = match gps_name {
        Some(name) => {
            let gps = GPS::new(name, serial_port_settings);

            match gps.run().await {
                Ok(()) => (),
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                }
            }

            Some(gps)
        },
        None => None,
    };

    let pps = match pps_name {
        Some(name) => {
            let mut pps = PPS::new(name);

            match pps.run().await {
                Ok(()) => (),
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                },
            };

            Some(pps)
        },
        None => None,
    };

    let mut gpsd = GpsdServer::new(2947);

    if let Some(g) = gps {
        gpsd.add_gps(g);
    }

    if let Some(p) = pps {
        gpsd.add_pps(p);
    }

    gpsd.run().await.unwrap();
}
