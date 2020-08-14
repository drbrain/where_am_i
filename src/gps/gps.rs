use crate::gps::GPSData;
use crate::nmea::*;
use crate::JsonSender;

use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

type Locked = Arc<Mutex<GPSData>>;

#[derive(Debug)]
pub struct GPS {
    pub name: String,
    pub tx: JsonSender,
    device_tx: Sender<NMEA>,
    data: Locked,
}

impl GPS {
    pub fn new(name: String, device_tx: Sender<NMEA>) -> Self {
        let (tx, _) = broadcast::channel(5);
        let data = GPSData::default();
        let data = Mutex::new(data);
        let data = Arc::new(data);

        GPS {
            name,
            tx,
            device_tx,
            data,
        }
    }

    pub async fn read(&mut self) {
        let data = Arc::clone(&self.data);
        let name = self.name.clone();
        let rx = self.device_tx.subscribe();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            read_device(rx, data, name, tx).await;
        });
    }
}

async fn read_device(mut rx: Receiver<NMEA>, data: Locked, name: String, tx: JsonSender) {
    let mut data = data.lock().await;

    while let Ok(nmea) = rx.recv().await {
        data.read_nmea(nmea, &name, &tx);
    }
}
