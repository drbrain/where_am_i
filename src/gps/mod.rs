mod driver;
mod generic;
mod gps_data;
mod gps_type;
mod mkt;
mod ublox_nmea;

pub use driver::add_message;
pub use driver::Driver;
pub use generic::Generic;
pub use gps_data::GPSData;
pub use gps_type::GpsType;
pub use mkt::MKTData;
pub use mkt::MKT;
pub use ublox_nmea::UBXConfig;
pub use ublox_nmea::UBXData;
pub use ublox_nmea::UBXNavigationStatus;
pub use ublox_nmea::UBXPort;
pub use ublox_nmea::UBXPortMask;
pub use ublox_nmea::UBXPosition;
pub use ublox_nmea::UBXPositionPoll;
pub use ublox_nmea::UBXRate;
pub use ublox_nmea::UBXSatellite;
pub use ublox_nmea::UBXSatelliteStatus;
pub use ublox_nmea::UBXSatellites;
pub use ublox_nmea::UBXSvsPoll;
pub use ublox_nmea::UBXTime;
pub use ublox_nmea::UBXTimePoll;
pub use ublox_nmea::UBloxNMEA;

use crate::configuration::GpsConfig;
use crate::gpsd::Response;
use crate::nmea::Device;
use crate::nmea::*;
use crate::TSSender;
use anyhow::Result;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;

type Locked = Arc<Mutex<GPSData>>;

#[derive(Clone, Debug)]
pub struct GPS {
    pub name: String,
    pub gpsd_tx: broadcast::Sender<Response>,
    pub ntp_tx: TSSender,
    device: Device,
    data: Locked,
}

impl GPS {
    pub async fn new(config: &GpsConfig) -> Result<Self> {
        let device = Device::new(&config).await?;

        let name = config.name.clone();
        let (gpsd_tx, _) = broadcast::channel(5);
        let (ntp_tx, _) = broadcast::channel(5);
        let data = Arc::new(Mutex::new(GPSData::default()));

        Ok(GPS {
            name,
            gpsd_tx,
            ntp_tx,
            device,
            data,
        })
    }

    pub fn subscribe_nmea(&self) -> broadcast::Receiver<NMEA> {
        self.device.subscribe()
    }

    pub fn start(&self) {
        let data = Arc::clone(&self.data);
        let name = self.name.clone();
        let rx = self.device.subscribe();
        let gpsd_tx = self.gpsd_tx.clone();
        let ntp_tx = self.ntp_tx.clone();

        tokio::spawn(async move {
            read_device(rx, data, name, gpsd_tx, ntp_tx).await;
        });
    }
}

async fn read_device(
    mut rx: Receiver<NMEA>,
    data: Locked,
    name: String,
    gpsd_tx: broadcast::Sender<Response>,
    ntp_tx: TSSender,
) {
    let mut data = data.lock().await;

    while let Ok(nmea) = rx.recv().await {
        data.read_nmea(nmea, &name, &gpsd_tx, &ntp_tx);
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_mkt;

#[cfg(test)]
mod test_ublox_nmea;
