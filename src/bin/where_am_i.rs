mod args;

use where_am_i::gps::GPS;
use where_am_i::gpsd::Server;
use where_am_i::nmea::Device;
use where_am_i::nmea::UBX_OUTPUT_MESSAGES;
use where_am_i::pps::PPS;
use where_am_i::shm::NtpShm;

use tokio::runtime;

use tracing::error;
use tracing::info;
use tracing::Level;

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

    let (gps_name, serial_port_settings, pps_name) = args::where_am_i_args();

    let gps = match gps_name.clone() {
        Some(name) => {
            let mut device = Device::new(name.clone(), serial_port_settings);

            for default in &UBX_OUTPUT_MESSAGES {
                device.message(&default.to_string(), false);
            }

            device.message(&"ZDA".to_string(), true);

            let device_tx = match device.run().await {
                Ok(tx) => tx,
                Err(e) => {
                    error!("failed to read from GPS: {:?}", e);
                    std::process::exit(1);
                }
            };

            let mut gps = GPS::new(name, device_tx);

            gps.read().await;

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

    let mut ntp_shm = NtpShm::new(2);
    let mut server = Server::new(2947);

    if let Some(g) = gps {
        ntp_shm.add_gps(g.tx.clone());
        server.add_gps(g);
        info!("registered GPS");
    }

    if let Some(p) = pps {
        let device_name = match gps_name {
            Some(n) => n,
            None => p.name.clone(),
        };

        ntp_shm.add_pps(p.tx.clone());
        server.add_pps(p, device_name);
        info!("registered PPS");
    }

    ntp_shm.run().await;
    server.run().await.unwrap();
}
