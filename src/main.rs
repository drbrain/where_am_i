mod args;
mod gps;
mod pps;
mod serial;
mod server;

#[macro_use] extern crate nix;

#[macro_use]
extern crate nom;

use tokio::runtime;
use tokio::sync::broadcast;

use tracing::Level;
use tracing::error;

use tracing_subscriber;

pub type JsonReceiver = broadcast::Receiver<String>;
pub type JsonSender = broadcast::Sender<String>;

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
    let (tx, _) = broadcast::channel(1);

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("no global subscriber has been set");

    let (gps_name, serial_port_settings, pps_name) = args::parse();

    match gps_name {
        Some(name) => gps::spawn(name, serial_port_settings, tx.clone()),
        None       => (),
    };

    match pps_name {
        Some(name) => {
            match pps::spawn(name, tx.clone()) {
                Ok(()) => (),
                Err(e) => {
                    error!("unable to watch PPS events: {}", e);

                    std::process::exit(1);
                },
            };
        },
        None       => (),
    };

    server::spawn(2947, tx.clone()).await;
}
