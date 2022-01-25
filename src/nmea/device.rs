use crate::configuration::GpsConfig;
use crate::nmea::Codec;
use crate::nmea::DeviceBuilder;
use crate::nmea::NMEA;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;

pub type SerialCodec = Framed<SerialStream, Codec>;

#[derive(Debug)]
pub struct Device {
    pub name: String,
    pub(crate) sender: Arc<broadcast::Sender<NMEA>>,
}

impl Device {
    pub async fn new(config: &GpsConfig) -> Result<Self> {
        DeviceBuilder::new(config)?.build().await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<NMEA> {
        self.sender.subscribe()
    }
}
