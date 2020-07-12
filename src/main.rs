mod server;

use argh::FromArgs;

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

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;
use tokio_serial::StopBits;

pub type JsonQueue = broadcast::Sender<json::JsonValue>;

#[derive(FromArgs)]
/// Where am I?
struct Args {
    /// GPS baud rate
    #[argh(option, default = "default_baud()")]
    baud_rate: u32,

    /// GPS data bits
    #[argh(option, default = "default_bits()")]
    data_bits: u8,

    /// GPS parity
    #[argh(option, default = "default_parity()")]
    parity: String,

    /// GPS stop bits
    #[argh(option, default = "default_stop_bits()")]
    stop_bits: u8,

    /// GPS flow control
    #[argh(option, default = "default_flow_control()")]
    flow_control: String,

    /// device
    #[argh(positional)]
    device: String,
}

fn default_baud()         -> u32    { 38400 }
fn default_bits()         -> u8     { 8 }
fn default_flow_control() -> String { "none".to_string() }
fn default_parity()       -> String { "none".to_string() }
fn default_stop_bits()    -> u8     { 1 }

fn data_bits_from_int(i: u8) -> Result<DataBits, String> {
    match i {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        e => Err(format!("invalid data bits {}", e)),
    }
}

fn flow_control_from_str(s: String) -> Result<FlowControl, String> {
    match s.to_lowercase().as_str() {
        "n"        => Ok(FlowControl::None),
        "none"     => Ok(FlowControl::None),
        "hardware" => Ok(FlowControl::Hardware),
        "software" => Ok(FlowControl::Software),
        e => Err(format!("invalid flow control {}", e)),
    }
}

fn parity_from_str(s: String) -> Result<Parity, String> {
    match s.to_lowercase().as_str() {
        "e"    => Ok(Parity::Even),
        "even" => Ok(Parity::Even),
        "n"    => Ok(Parity::None),
        "none" => Ok(Parity::None),
        "o"    => Ok(Parity::Odd),
        "odd"  => Ok(Parity::Odd),
        e      => Err(format!("invalid parity {}", e)),
    }
}

fn stop_bits_from_str(i: u8) -> Result<StopBits, String> {
    match i {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        e => Err(format!("invalid stop bits {}", e)),
    }
}

#[tokio::main]
async fn main() {
    let args: Args = argh::from_env();

    let (name, serial_port_settings) = convert_args(args);

    let gps = open_gps(name, serial_port_settings).await;

    let time_tx = server::spawn(2947);

    let done_rx = spawn_parser(gps, time_tx);

    done_rx.await.unwrap();
}

fn convert_args(args: Args) -> (String, SerialPortSettings) {
    let s = SerialPortSettings {
        baud_rate:    args.baud_rate,
        data_bits:    data_bits_from_int(args.data_bits).unwrap(),
        flow_control: flow_control_from_str(args.flow_control).unwrap(),
        parity:       parity_from_str(args.parity).unwrap(),
        stop_bits:    stop_bits_from_str(args.stop_bits).unwrap(),
        timeout:      Duration::from_millis(1),
    };

    return (args.device, s);
}

async fn open_gps(device: String, settings: SerialPortSettings) -> BufReader<Serial> {
    let sp = match Serial::from_path(&device, &settings) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error {}", e);
            std::process::exit(1);
        }
    };

    let gps = BufReader::new(sp);

    return gps;
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
