extern crate json;

use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;

use json::object;
use json::parse;
use json::stringify;

use nmea::Nmea;
use nmea::ParseResult;

use std::env;
use std::time::Duration;
use std::time::SystemTime;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::BufWriter;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::net::tcp::WriteHalf;
use tokio::prelude::*;
use tokio::sync::broadcast;
use tokio::sync::oneshot;

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;
use tokio_serial::StopBits;

type Queue = broadcast::Sender<json::JsonValue>;

#[tokio::main]
async fn main() {
    let gps = open_gps().await;

    let time_tx = spawn_server(2947);

    let done_rx = spawn_parser(gps, time_tx);

    done_rx.await.unwrap();
}

async fn open_gps() -> BufReader<Serial> {
    let name = env::args().nth(1);

    if name.is_none() {
        eprintln!("Provide GPS device as first argument");
        std::process::exit(1);
    }

    let name = name.unwrap();

    let s = SerialPortSettings {
        baud_rate:    38400,
        data_bits:    DataBits::Eight,
        flow_control: FlowControl::None,
        parity:       Parity::None,
        stop_bits:    StopBits::One,
        timeout:      Duration::from_millis(1),
    };

    let sp = match Serial::from_path(&name, &s) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error {}", e);
            std::process::exit(1);
        }
    };

    let gps = BufReader::new(sp);

    return gps;
}

async fn handle_client(mut socket: TcpStream, tx: Queue) {
    let (recv, send) = socket.split();

    let recv = BufReader::new(recv);
    let mut lines = recv.lines();

    let mut send = BufWriter::new(send);

    loop {
        let request = match lines.next_line().await {
            Ok(l) => l,
            Err(_) => break,
        };

        if request.is_none() {
            break;
        }

        let request = request.unwrap();

        if request == "?VERSION;".to_string() {
            let version = json::object!{
                class: "VERSION",
                release: "where_am_i 0.0.0",
                rev: "",
                proto_major: 3,
                proto_minor: 10,
            };

            let message = format!("{}\n", stringify(version));

            let result = send.write(message.as_bytes()).await;

            if result.is_err() {
                break;
            }
        } else if request.starts_with("?WATCH=") {
            let json_start = match request.find("{") {
                Some(i) => i,
                None    => continue,
            };

            let json_end = match request.rfind("}") {
                Some(i) => i,
                None    => continue,
            };

            let watch_json = match request.get(json_start..=json_end) {
                Some(j) => j,
                None    => continue,
            };

            let watch = match parse(watch_json) {
                Ok(w) => w,
                Err(_) => {
                    eprintln!("Error parsing WATCH body {}", watch_json);
                    continue;
                },
            };

            let _device = watch["device"].as_str().unwrap_or_else(|| "UNSET");

            let enable = watch["enable"].as_bool().unwrap_or_else(|| false);

            let pps = watch["pps"].as_bool().unwrap_or_else(|| false);

            if enable && pps {
                relay_time_messages(&mut send, tx.clone()).await;
            }
        }

        let flushed = send.flush().await;
        if flushed.is_err() {
            break;
        }
    }
}

async fn relay_time_messages(send: &mut BufWriter<WriteHalf<'_>>, tx: Queue) {
    let mut rx = tx.subscribe();

    loop {
        let toff = match rx.recv().await {
            Ok(t) => t,
            Err(_) => break,
        };

        let message = format!("{}\n", stringify(toff));

        match send.write(message.as_bytes()).await {
            Ok(_) => (),
            Err(_) => break,
        };

        let flushed = send.flush().await;
        if flushed.is_err() {
            break;
        }
    }
}
fn spawn_server(port: u16) -> Queue {
    let (tx, _) = broadcast::channel(5);
    let time_tx = tx.clone();

    let address = ("0.0.0.0", port);

    tokio::spawn(async move {
        let mut listener = TcpListener::bind(address).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();

            let nodelay = socket.set_nodelay(true);

            if nodelay.is_err() {
                continue;
            }

            handle_client(socket, time_tx.clone()).await;
        }
    });

    return tx;
}

fn spawn_parser(input: BufReader<Serial>, time_tx: Queue) -> oneshot::Receiver<bool> {
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

fn report_time(rmc: nmea::RmcData, received: Duration, time_tx: &Queue) {
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
