use crate::{
    configuration::GpsConfig,
    gps::{Driver, Generic, GpsType, UBloxNMEA, MKT},
    nmea::{Codec, Device, MessageSetting, NMEA},
};
use anyhow::{Context, Result};
use backoff::{ExponentialBackoff, SystemClock};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use prometheus::{register_int_counter_vec, IntCounterVec};
use std::{
    convert::TryFrom,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tokio_serial::{SerialPortBuilder, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, info_span, Instrument};

lazy_static! {
    static ref DEVICE_OPENS: IntCounterVec = register_int_counter_vec!(
        "where_am_i_device_opens_count",
        "Count of times a device was open with result",
        &["device", "result"]
    )
    .unwrap();
    static ref NMEA_MESSAGES: IntCounterVec = register_int_counter_vec!(
        "where_am_i_nmea_messages_read_count",
        "Count of NMEA messages read from a device",
        &["device"]
    )
    .unwrap();
    static ref NMEA_ERRORS: IntCounterVec = register_int_counter_vec!(
        "where_am_i_nmea_read_errors_count",
        "Count of NMEA errors when reading from a device",
        &["device"]
    )
    .unwrap();
}

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
        let span_name = self.device.clone();
        let (sender, _) = broadcast::channel(20);
        let task_sender = sender.clone();
        let sender = Arc::new(sender);

        tokio::task::spawn(async move {
            let span = info_span!("device", name = span_name.as_str());

            self.start(task_sender).instrument(span).await
        });

        Ok(Device { name, sender })
    }

    async fn open(&self) -> Result<SerialStream> {
        backoff::future::retry(self.backoff.clone(), || async {
            let serial = self
                .serial_port_builder
                .clone()
                .open_native_async()
                .map_err(|e| log_error(&self.device, e))
                .with_context(|| format!("Failed to open GPS device {}", self.device))?;

            DEVICE_OPENS
                .with_label_values(&[&self.device, "success"])
                .inc();

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

            let nmea_messages = NMEA_MESSAGES.with_label_values(&[&self.device]);
            let nmea_errors = NMEA_ERRORS.with_label_values(&[&self.device]);

            // send NMEA messages
            loop {
                match framed.next().await {
                    Some(Ok(nmea)) => {
                        nmea_messages.inc();
                        sender.send(nmea).unwrap_or(0);
                    }
                    Some(Err(e)) => {
                        nmea_errors.inc();
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

fn log_error<T: std::fmt::Display>(device: &str, e: T) -> T {
    error!("Opening {device} failed: {e}");

    DEVICE_OPENS.with_label_values(&[&device, "failed"]).inc();

    e
}
