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
use tokio::sync::oneshot;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

type NMEASender = broadcast::Sender<NMEA>;
type RestartWaiter = oneshot::Sender<()>;
type SerialCodec = Framed<Serial, Codec>;

#[derive(Clone, Debug)]
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

    pub fn message(&mut self, id: &str, enabled: bool) {
        let setting = MessageSetting {
            id: id.to_string(),
            enabled,
        };

        self.messages.push(setting);
    }

    pub async fn run(&self) -> Result<NMEASender, io::Error> {
        let name = self.name.clone();
        let settings = self.settings.clone();
        let messages = self.messages.clone();
        let reader_tx = self.sender.clone();

        tokio::spawn(async move {
            start(&name, &settings, messages, reader_tx).await;
        });

        Ok(self.sender.clone())
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

async fn configure_device(serial: &mut SerialCodec, messages: Vec<MessageSetting>) {
    for message in messages {
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

async fn read_nmea(mut reader: SerialCodec, tx: NMEASender, restarter: RestartWaiter) {
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

                match restarter.send(()) {
                    Ok(_) => (),
                    Err(e) => error!("error notifying device it needs to restart: {:?}", e),
                };

                return;
            }
        }
    }
}

async fn send_messages(reader_tx: NMEASender, serial: SerialCodec, restarter: RestartWaiter) {
    tokio::spawn(async move {
        read_nmea(serial, reader_tx, restarter).await;
    });
}

async fn start(
    name: &str,
    settings: &SerialPortSettings,
    messages: Vec<MessageSetting>,
    reader_tx: NMEASender,
) {
    loop {
        let (restarter, waiter) = oneshot::channel();

        let mut serial = match open(&name, &settings).await {
            Ok(s) => s,
            Err(_) => unreachable!("open retries opening the device forever"),
        };

        configure_device(&mut serial, messages.clone()).await;

        send_messages(reader_tx.clone(), serial, restarter).await;

        waiter.await.unwrap_or_default();

        info!("Device hung up, retrying");
    }
}
