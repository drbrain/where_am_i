use crate::nmea::Codec;
use crate::nmea::NMEA;

use futures_util::stream::StreamExt;

use std::io;

use tokio::sync::broadcast;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

use tokio_util::codec::FramedRead;

use tracing::debug;
use tracing::error;

pub struct Device {
    pub name: String,
    settings: SerialPortSettings,
}

impl Device {
    pub fn new(name: String, settings: SerialPortSettings) -> Self {
        Device { name, settings }
    }

    pub async fn run(&self) -> Result<broadcast::Sender<NMEA>, io::Error> {
        let (tx, _) = broadcast::channel(20);

        let reader_tx = tx.clone();

        let serial = match Serial::from_path(self.name.clone(), &self.settings) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        debug!("Opened NMEA device {}", self.name);

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
                    error!("error sending device result: {:?}", e);
                    return;
                }
            },
            Err(e) => {
                error!("error reading from device: {:?}", e);
                return;
            }
        }
    }
}
