use chrono::prelude::*;

use crate::nmea::*;
use crate::JsonSender;

use serde_json::json;

use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;

use tracing::error;
use tracing::info;

type Locked = Arc<Mutex<GPSdata>>;
type Unlocked<'a> = MutexGuard<'a, GPSdata>;

#[derive(Debug, Default)]
pub struct GPSdata {
    pub time: Option<DateTime<Utc>>,
}

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
        let data = GPSdata::default();
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
        read_nmea(nmea, &mut data, &name, &tx);
    }
}

fn read_nmea(nmea: NMEA, data: &mut Unlocked, name: &String, tx: &JsonSender) {
    match nmea {
        NMEA::InvalidChecksum(cm) => error!(
            "checksum match, given {}, calculated {} on {}",
            cm.given, cm.calculated, cm.message
        ),
        NMEA::ParseError(e) => error!("parse error: {}", e),
        NMEA::ParseFailure(f) => error!("parse failure: {}", f),
        NMEA::Unsupported(n) => error!("unsupported: {}", n),
        NMEA::ZDA(nd) => zda(nd, data, name, tx),
        _ => (),
    }
}

fn zda(zda: ZDAdata, data: &mut Unlocked, name: &String, tx: &JsonSender) {
    let date = NaiveDate::from_ymd(zda.year, zda.month, zda.day);
    let time = NaiveDateTime::new(date, zda.time);
    let time = DateTime::from_utc(time, Utc);

    info!("{}", time);

    data.time = Some(time);

    report_time(time, name, tx);
}

#[tracing::instrument]
fn report_time(date: DateTime<Utc>, name: &String, tx: &JsonSender) {
    let sec = date.timestamp();
    let nsec = date.timestamp_subsec_nanos();

    // move this up
    let received = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n,
        Err(_) => return,
    };

    let toff = json!({
        "class":      "TOFF".to_string(),
        "device":     name,
        "real_sec":   sec,
        "real_nsec":  nsec,
        "clock_sec":  received.as_secs(),
        "clock_nsec": received.subsec_nanos(),
    });

    if tx.send(toff).is_ok() {}
}
