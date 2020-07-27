use crate::JsonSender;
use crate::serial;

use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;

use serde_json::json;

use std::time::Duration;
use std::time::SystemTime;

use nmea::Nmea;
use nmea::ParseResult;

use tokio::prelude::*;

use tokio_serial::SerialPortSettings;

use tracing::error;

#[tracing::instrument]
pub fn spawn(device: String, settings: SerialPortSettings, tx: JsonSender) {
    tokio::spawn(async move {
        let gps = serial::open(device, settings).await;

        let mut lines = gps.lines();

        let mut nmea = Nmea::new();

        loop {
            let line = match lines.next_line().await {
                Ok(l)  => l,
                Err(e) => {
                    error!("Failed to read from GPS ({:?})", e);
                    break;
                }
            };

            let received = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let line = match line {
                Some(l) => l,
                None => {
                    error!("No line from GPS");
                    break;
                }
            };

            let parsed = nmea.parse(&line);

            if parsed.is_err() {
                //error!("Failed to parse {} ({:?})", line, parsed.err());
                continue;
            }

            match parsed.unwrap() {
                ParseResult::RMC(rmc) => report_time(rmc, received, &tx),
                _ => (),
            };
        }
    });
}

#[tracing::instrument]
fn report_time(rmc: nmea::RmcData, received: Duration, tx: &JsonSender) {
    let time = rmc.fix_time;
    if time.is_none() {
        return;
    }

    let date = rmc.fix_date;
    if date.is_none() {
        return;
    }

    let ts = NaiveDateTime::new(date.unwrap(), time.unwrap());
    let timestamp = DateTime::<Utc>::from_utc(ts, Utc);

    let sec  = timestamp.timestamp();
    let nsec = timestamp.timestamp_subsec_nanos();

    let toff = json!({
        "class":      "TOFF".to_string(),
        "device":     "".to_string(),
        "real_sec":   sec,
        "real_nsec":  nsec,
        "clock_sec":  received.as_secs(),
        "clock_nsec": received.subsec_nanos(),
    });

    match tx.send(toff) {
        Ok(_)  => (),
        Err(_) => (), // error!("send error: {:?}", e),
    }
}

