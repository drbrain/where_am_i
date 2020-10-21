use crate::gps::GPSData;
use crate::nmea::*;
use crate::JsonSender;
use crate::TSSender;

use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

type Locked = Arc<Mutex<GPSData>>;

#[derive(Debug)]
pub struct GPS {
    pub name: String,
    pub gpsd_tx: JsonSender,
    pub ntp_tx: TSSender,
    device_tx: Sender<NMEA>,
    data: Locked,
}

impl GPS {
    pub fn new(name: String, device_tx: Sender<NMEA>) -> Self {
        let (gpsd_tx, _) = broadcast::channel(5);
        let (ntp_tx, _) = broadcast::channel(5);
        let data = GPSData::default();
        let data = Mutex::new(data);
        let data = Arc::new(data);

        GPS {
            name,
            gpsd_tx,
            ntp_tx,
            device_tx,
            data,
        }
    }

    pub async fn read(&mut self) {
        let data = Arc::clone(&self.data);
        let name = self.name.clone();
        let rx = self.device_tx.subscribe();
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
    gpsd_tx: JsonSender,
    ntp_tx: TSSender,
) {
    let mut data = data.lock().await;

    while let Ok(nmea) = rx.recv().await {
        data.read_nmea(nmea, &name, &gpsd_tx, &ntp_tx);
    }
}
