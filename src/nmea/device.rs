use anyhow::Context;
use anyhow::Result;

use backoff::ExponentialBackoff;
use backoff::SystemClock;

use crate::gps::Driver;
use crate::gps::Generic;
use crate::gps::GpsType;
use crate::gps::UBloxNMEA;
use crate::gps::MKT;
use crate::nmea::Codec;
use crate::nmea::NMEA;

use futures_util::StreamExt;

use instant::Instant;

use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::oneshot;

use tokio_serial::SerialPortBuilder;
use tokio_serial::SerialPortBuilderExt;
use tokio_serial::SerialStream;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

type NMEASender = broadcast::Sender<NMEA>;
type RestartWaiter = oneshot::Sender<()>;
pub type SerialCodec = Framed<SerialStream, Codec>;

#[derive(Clone, Debug)]
pub struct MessageSetting {
    pub id: String,
    pub enabled: bool,
}

pub struct Device {
    pub name: String,
    pub sender: NMEASender,
    gps_type: GpsType,
    serial_port_builder: SerialPortBuilder,
    messages: Vec<MessageSetting>,
}

impl Device {
    pub fn new(name: String, gps_type: GpsType, serial_port_builder: SerialPortBuilder) -> Self {
        let messages = vec![];

        let (sender, _) = broadcast::channel(20);

        Device {
            name,
            sender,
            gps_type,
            serial_port_builder,
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

    pub async fn open(&self) -> Result<(SerialStream, Driver)> {
        open(&self.name, &self.gps_type, &self.serial_port_builder).await
    }

    pub async fn run(&self) -> NMEASender {
        let name = self.name.clone();
        let serial_port_builder = self.serial_port_builder.clone();
        let gps_type = self.gps_type.clone();
        let messages = self.messages.clone();
        let reader_tx = self.sender.clone();

        tokio::spawn(async move {
            start(&name, &gps_type, &serial_port_builder, messages, reader_tx).await;
        });

        self.sender.clone()
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

async fn open(
    name: &str,
    gps_type: &GpsType,
    serial_port_builder: &SerialPortBuilder,
) -> Result<(SerialStream, Driver)> {
    let driver = match gps_type {
        GpsType::UBlox => Driver::UBloxNMEA(UBloxNMEA::default()),
        GpsType::MKT => Driver::MKT(MKT::default()),
        GpsType::Generic => Driver::Generic(Generic::default()),
    };

    backoff::future::retry(backoff(), || async {
        let serial = serial_port_builder
            .clone()
            .open_native_async()
            .map_err(open_error)
            .with_context(|| format!("Failed to open GPS device {}", name.clone()))?;

        debug!("Opened NMEA device {}", name.clone());

        Ok((serial, driver.clone()))
    })
    .await
}

fn open_error(e: tokio_serial::Error) -> tokio_serial::Error {
    error!("Opening failed: {}", e.to_string());

    e
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
    gps_type: &GpsType,
    serial_port_builder: &SerialPortBuilder,
    messages: Vec<MessageSetting>,
    reader_tx: NMEASender,
) {
    loop {
        let (restarter, waiter) = oneshot::channel();

        let (serial, driver) = match open(&name, &gps_type, &serial_port_builder).await {
            Ok(t) => t,
            Err(_) => unreachable!("open retries opening the device forever"),
        };

        let mut framed = Framed::new(serial, Codec::new(driver.clone()));

        driver.configure(&mut framed, messages.clone()).await;

        send_messages(reader_tx.clone(), framed, restarter).await;

        waiter.await.unwrap_or_default();

        info!("Device hung up, retrying");
    }
}
