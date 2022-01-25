use crate::configuration::GpsConfig;
use crate::gps::Driver;
use crate::gps::Generic;
use crate::gps::GpsType;
use crate::gps::UBloxNMEA;
use crate::gps::MKT;
use crate::nmea::Codec;
use crate::nmea::Device;
use crate::nmea::MessageSetting;
use crate::nmea::NMEA;
use anyhow::Context;
use anyhow::Result;
use backoff::ExponentialBackoff;
use backoff::SystemClock;
use futures_util::StreamExt;
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio_serial::SerialPortBuilder;
use tokio_serial::SerialPortBuilderExt;
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;
use tracing::debug;
use tracing::error;
use tracing::info;

pub struct DeviceBuilder {
    device: String,
    driver: Driver,
    backoff: ExponentialBackoff,
    serial_port_builder: SerialPortBuilder,
    message_settings: Vec<MessageSetting>,
}

impl DeviceBuilder {
    pub fn new(config: &GpsConfig) -> Result<Self> {
        let device = config.device.clone();
        let serial_port_builder = SerialPortBuilder::try_from(config.clone())?;

        let driver = match config.gps_type {
            GpsType::UBlox => Driver::UBloxNMEA(UBloxNMEA::default()),
            GpsType::MKT => Driver::MKT(MKT::default()),
            GpsType::Generic => Driver::Generic(Generic::default()),
        };

        let message_settings = driver.message_settings(&config.messages());

        Ok(DeviceBuilder {
            device,
            driver,
            backoff: default_backoff(),
            serial_port_builder,
            message_settings,
        })
    }

    pub async fn build(self) -> Result<Device> {
        let name = self.device.clone();
        let (sender, _) = broadcast::channel(20);
        let task_sender = sender.clone();
        let sender = Arc::new(sender);

        tokio::task::spawn(async move { self.start(task_sender).await });

        Ok(Device { name, sender })
    }

    async fn open(&self) -> Result<SerialStream> {
        backoff::future::retry(self.backoff.clone(), || async {
            let serial = self
                .serial_port_builder
                .clone()
                .open_native_async()
                .map_err(log_error)
                .with_context(|| format!("Failed to open GPS device {}", self.device))?;

            debug!("Opened NMEA serial port {}", self.device);

            Ok(serial)
        })
        .await
    }

    async fn start(&self, sender: broadcast::Sender<NMEA>) {
        loop {
            let serial = match self.open().await {
                Ok(t) => t,
                Err(_) => unreachable!("open retries opening the device forever"),
            };

            let mut framed = Framed::new(serial, Codec::new(self.driver.clone()));

            self.driver
                .configure(&mut framed, &self.message_settings)
                .await;

            // send NMEA messages
            loop {
                match framed.next().await {
                    Some(Ok(nmea)) => {
                        sender.send(nmea).unwrap_or(0);
                    }
                    Some(Err(e)) => {
                        error!("NMEA device {} parse error {:?}", self.device, e);
                        break;
                    }
                    None => {
                        error!("NMEA device {} has no more messages", self.device);
                        break;
                    }
                };
            }

            info!("Device {} hung up, retrying", self.device);
        }
    }
}

fn default_backoff() -> ExponentialBackoff {
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

fn log_error<T: std::fmt::Display>(e: T) -> T {
    error!("Opening failed: {}", e);

    e
}
