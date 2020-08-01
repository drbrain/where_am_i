use crate::JsonSender;

use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;

use serde_json::json;

use std::fmt;
use std::io;
use std::time::Duration;
use std::time::SystemTime;

use nmea::Nmea;
use nmea::ParseResult;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

use tracing::error;
use tracing::info;

pub struct GPS {
    pub name: String,
    pub tx: JsonSender,
    settings: SerialPortSettings,
}

impl GPS {
    pub fn new(name: String, settings: SerialPortSettings) -> Self {
        let (tx, _) = broadcast::channel(5);

        GPS { name, tx, settings }
    }

    #[tracing::instrument]
    pub async fn run(&self) -> Result<(), io::Error> {
        let (mut result_tx, mut result_rx) = mpsc::channel(1);
        let name = self.name.clone();
        let settings = self.settings;
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let serial = match Serial::from_path(name.clone(), &settings) {
                Ok(s) => {
                    result_tx.send(Ok(())).await.unwrap();
                    s
                }
                Err(e) => {
                    result_tx.send(Err(e)).await.unwrap();
                    return;
                }
            };

            let mut lines = BufReader::new(serial).lines();

            info!("Opened GPS device {}", name);

            let mut nmea = Nmea::new();

            loop {
                let line = match lines.next_line().await {
                    Ok(l) => l,
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

                if let ParseResult::RMC(rmc) = parsed.unwrap() {
                    report_time(rmc, name.clone(), received, &tx);
                }
            }
        });

        match result_rx.recv().await {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()), // Should raise a different error
        }
    }
}

#[tracing::instrument]
fn report_time(rmc: nmea::RmcData, name: String, received: Duration, tx: &JsonSender) {
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

    let sec = timestamp.timestamp();
    let nsec = timestamp.timestamp_subsec_nanos();

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

impl fmt::Debug for GPS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GPS")
            .field("name", &self.name)
            .field("tx", &self.tx)
            .finish()
    }
}
