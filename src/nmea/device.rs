use backoff::future::FutureOperation;
use backoff::ExponentialBackoff;
use backoff::SystemClock;

use crate::nmea::Codec;
use crate::nmea::UBXRate;
use crate::nmea::NMEA;

use instant::Instant;

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use std::io;
use std::time::Duration;

use tokio::sync::broadcast;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

type NMEASender = broadcast::Sender<NMEA>;
type SerialCodec = Framed<Serial, Codec>;

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
    pub sender: NMEASender,
    settings: SerialPortSettings,
    messages: Vec<MessageSetting>,
}

impl Device {
    pub fn new(name: String, settings: SerialPortSettings) -> Self {
        let messages = vec![];

        let (sender, _) = broadcast::channel(20);

        Device {
            name,
            settings,
            messages,
            sender,
        }
    }

    async fn configure_device(&self, serial: &mut SerialCodec) {
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
    }

    pub fn message(&mut self, id: &str, enabled: bool) {
        let setting = MessageSetting {
            id: id.to_string(),
            enabled,
        };

        self.messages.push(setting);
    }

    pub async fn run(&self) -> Result<NMEASender, io::Error> {
        let mut serial = match open(&self.name, &self.settings).await {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        self.configure_device(&mut serial).await;

        self.send_messages(serial).await;

        Ok(self.sender.clone())
    }

    async fn send_messages(&self, serial: SerialCodec) {
        let reader_tx = self.sender.clone();

        tokio::spawn(async move {
            read_nmea(serial, reader_tx).await;
        });
    }
}

fn backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        current_interval: Duration::from_millis(50),
        initial_interval: Duration::from_millis(50),
        randomization_factor: 0.25,
        multiplier: 1.5,
        max_interval: Duration::from_millis(60_000),
        max_elapsed_time: None,
        clock: SystemClock::default(),
        start_time: Instant::now(),
    }
}

async fn open(name: &str, settings: &SerialPortSettings) -> Result<SerialCodec, io::Error> {
    (|| async {
        let serial = Serial::from_path(name.clone(), &settings).map_err(open_error)?;

        debug!("Opened NMEA device {}", name.clone());

        Ok(Framed::new(serial, Codec::default()))
    })
    .retry(backoff())
    .await
}

fn open_error(e: io::Error) -> io::Error {
    error!("Opening failed: {}", e.to_string());

    e
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

async fn read_nmea(mut reader: SerialCodec, tx: NMEASender) {
    loop {
        let nmea = match reader.next().await {
            Some(n) => n,
            None => {
                error!("GPS device has no more messages");
                return;
            }
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
