mod args;
mod serial;
mod server;

use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;

use json::object;

use nmea::Nmea;
use nmea::ParseResult;

use std::time::Duration;
use std::time::SystemTime;

use tokio::io::BufReader;
use tokio::prelude::*;
use tokio::sync::broadcast;
use tokio::sync::oneshot;

use tokio_serial::Serial;

pub type JsonQueue = broadcast::Sender<json::JsonValue>;

#[tokio::main]
async fn main() {
    let (name, serial_port_settings) = args::parse();

    let gps = serial::open(name, serial_port_settings).await;

    let time_tx = server::spawn(2947);

    let done_rx = spawn_parser(gps, time_tx);

    done_rx.await.unwrap();
}

fn spawn_parser(input: BufReader<Serial>, time_tx: JsonQueue) -> oneshot::Receiver<bool> {
    let mut lines = input.lines();

    let mut nmea = Nmea::new();
    let (done_tx, done_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            let line = match lines.next_line().await {
                Ok(l)  => l,
                Err(e) => {
                    eprintln!("Failed to read from GPS ({:?})", e);
                    done_tx.send(false).unwrap();
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
                    eprintln!("No line from GPS");
                    done_tx.send(false).unwrap();
                    break;
                }
            };

            let parsed = nmea.parse(&line);

            if parsed.is_err() {
                eprintln!("Failed to parse {} ({:?})", line, parsed.err());
                continue;
            }

            match parsed.unwrap() {
                ParseResult::RMC(rmc) => report_time(rmc, received, &time_tx),
                _ => (),
            };
        }
    });

    return done_rx;
}

fn report_time(rmc: nmea::RmcData, received: Duration, time_tx: &JsonQueue) {
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

    let toff = object! {
        class:      "TOFF".to_string(),
        device:     "".to_string(),
        real_sec:   sec,
        real_nsec:  nsec,
        clock_sec:  received.as_secs(),
        clock_nsec: received.subsec_nanos(),
    };

    match time_tx.send(toff) {
        Ok(_)  => (),
        Err(_) => (),
    }
}
