mod args;

use tracing::error;
use tracing::info;
use tracing::Level;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    let (gps_name, serial_port_settings) = args::gps_watch_args();

    let gps = gps::GPS::new(gps_name, serial_port_settings);

    let tx = match gps.run().await {
        Ok(t) => t,
        Err(e) => {
            error!("failed to read from GPS: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut rx = tx.subscribe();

    while let Ok(nmea) = rx.recv().await {
        info!("{:?}", nmea);
    }
}

mod gps {
    use where_am_i::nmea::Codec;
    use where_am_i::nmea::NMEA;

    use futures_util::stream::StreamExt;

    use std::io;

    use tokio::sync::broadcast;

    use tokio_serial::Serial;
    use tokio_serial::SerialPortSettings;

    use tokio_util::codec::FramedRead;

    use tracing::debug;
    use tracing::error;

    pub struct GPS {
        pub name: String,
        settings: SerialPortSettings,
    }

    impl GPS {
        pub fn new(name: String, settings: SerialPortSettings) -> Self {
            GPS { name, settings }
        }

        pub async fn run(&self) -> Result<broadcast::Sender<NMEA>, io::Error> {
            let (tx, _) = broadcast::channel(20);

            let reader_tx = tx.clone();

            let serial = match Serial::from_path(self.name.clone(), &self.settings) {
                Ok(s) => s,
                Err(e) => return Err(e),
            };

            debug!("Opened GPS {}", self.name);

            let reader = FramedRead::new(serial, Codec::new());

            tokio::spawn(async move {
                read_nmea(reader, reader_tx).await;
            });

            Ok(tx)
        }
    }

    async fn read_nmea(mut reader: FramedRead<Serial, Codec>, tx: broadcast::Sender<NMEA>) {
        loop {
            let nmea = match reader.next().await {
                Some(n) => n,
                None => return,
            };

            match nmea {
                Ok(n) => match tx.send(n) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("error sending GPS result: {:?}", e);
                        return;
                    }
                },
                Err(e) => {
                    error!("error reading from GPS: {:?}", e);
                    return;
                }
            }
        }
    }
}
