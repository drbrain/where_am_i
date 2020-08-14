use chrono::prelude::*;

use crate::nmea::*;
use crate::JsonSender;

use serde_json::json;

use std::time::SystemTime;

use tracing::error;

#[derive(Debug, Default)]
pub struct GPSData {
    pub time: Option<DateTime<Utc>>,
    pub year: i32,

    pub lat_lon: Option<LatLon>,
    pub altitude_msl: Option<f32>,

    pub quality: Quality,
}

impl GPSData {
    pub fn read_nmea(&mut self, nmea: NMEA, name: &str, tx: &JsonSender) {
        match nmea {
            NMEA::InvalidChecksum(cm) => error!(
                "checksum match, given {}, calculated {} on {}",
                cm.given, cm.calculated, cm.message
            ),
            NMEA::ParseError(e) => error!("parse error: {}", e),
            NMEA::ParseFailure(f) => error!("parse failure: {}", f),
            NMEA::Unsupported(n) => error!("unsupported: {}", n),
            NMEA::GGA(nd) => gga(self, nd, name, tx),
            NMEA::ZDA(nd) => zda(self, nd, name, tx),
            _ => (),
        }
    }
}

fn gga(data: &mut GPSData, gga: GGAData, name: &str, tx: &JsonSender) {
    data.quality = gga.quality;
    data.lat_lon = Some(gga.lat_lon);
    data.altitude_msl = Some(gga.alt);
}

fn zda(data: &mut GPSData, zda: ZDAData, name: &str, tx: &JsonSender) {
    let date = NaiveDate::from_ymd(zda.year, zda.month, zda.day);
    let time = NaiveDateTime::new(date, zda.time);
    let time = DateTime::from_utc(time, Utc);

    data.time = Some(time);
    data.year = time.year();

    report_time(time, name, tx);
}

#[tracing::instrument]
fn report_time(date: DateTime<Utc>, name: &str, tx: &JsonSender) {
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
