use crate::nmea::Codec;
use crate::nmea::UBXRate;
use crate::nmea::NMEA;

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use std::io;

use tokio::sync::broadcast;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

struct MessageSetting {
    id: String,
    enabled: bool,
}

pub const UBX_OUTPUT_MESSAGES: [&str; 15] = [
    "DTM", "GBS", "GGA", "GLL", "GNS", "GRS", "GSA", "GST", "GSV", "RLM", "RMC", "TXT", "VLW",
    "VTG", "ZDA",
];

pub struct Device {
    pub name: String,
    settings: SerialPortSettings,
    messages: Vec<MessageSetting>,
}

impl Device {
    pub fn new(name: String, settings: SerialPortSettings) -> Self {
        let messages = vec![];

        Device {
            name,
            settings,
            messages,
        }
    }

    pub fn message(&mut self, id: &str, enabled: bool) {
        let setting = MessageSetting {
            id: id.to_string(),
            enabled,
        };

        self.messages.push(setting);
    }

    pub async fn run(&self) -> Result<broadcast::Sender<NMEA>, io::Error> {
        let (tx, _) = broadcast::channel(20);

        let reader_tx = tx.clone();

        let serial = match Serial::from_path(self.name.clone(), &self.settings) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        debug!("Opened NMEA device {}", self.name);

        let mut serial = Framed::new(serial, Codec::default());

        for message in &self.messages {
            let rate = rate_for(message.id.clone(), message.enabled);

            match serial.send(rate).await {
                Ok(_) => info!("set {} to {}", message.id, message.enabled),
                Err(e) => error!(
                    "unable to set {} to {}: {:?}",
                    message.id, message.enabled, e
                ),
            }
        }

        tokio::spawn(async move {
            read_nmea(serial, reader_tx).await;
        });

        Ok(tx)
    }
}

fn rate_for(msg_id: String, enabled: bool) -> UBXRate {
    let rus1 = if enabled { 1 } else { 0 };

    UBXRate {
        message: msg_id,
        rddc: 0,
        rus1,
        rus2: 0,
        rusb: 0,
        rspi: 0,
        reserved: 0,
    }
}

async fn read_nmea(mut reader: Framed<Serial, Codec>, tx: broadcast::Sender<NMEA>) {
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
