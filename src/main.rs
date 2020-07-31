mod args;
mod gps;
mod gpsd;
mod pps;

#[macro_use]
extern crate nix;

use gps::GPS;
use gpsd::Server;
use pps::PPS;

use serde_json::Value;

use tokio::runtime;
use tokio::sync::broadcast;

use tracing::error;
use tracing::info;
use tracing::Level;

use tracing_subscriber;

pub type JsonReceiver = broadcast::Receiver<Value>;
pub type JsonSender = broadcast::Sender<Value>;

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

    let (gps_name, serial_port_settings, pps_name) = args::parse();

    let gps = match gps_name.clone() {
        Some(name) => {
            let gps = GPS::new(name.clone(), serial_port_settings);

            match gps.run().await {
                Ok(()) => (),
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                }
            }

            Some(gps)
        }
        None => None,
    };

    let pps = match pps_name {
        Some(name) => {
            let device_name = match gps_name.clone() {
                Some(n) => n,
                None => name.clone(),
            };

            let mut pps = PPS::new(name, device_name);

            match pps.run().await {
                Ok(()) => (),
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                }
            };

            Some(pps)
        }
        None => None,
    };

    let mut server = Server::new(2947);

    if let Some(g) = gps {
        server.add_gps(g);
        info!("registered GPS");
    }

    if let Some(p) = pps {
        let device_name = match gps_name {
            Some(n) => n,
            None => p.name.clone(),
        };

        server.add_pps(p, device_name.clone());
        info!("registered PPS");
    }

    server.run().await.unwrap();
}
