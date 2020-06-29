extern crate nmea;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let socket = open_socket();

    let mut nmea = nmea::Nmea::new();

    parse_loop(&mut nmea, socket);
}

fn open_socket() -> File {
    let name = env::args().nth(1);

    if name.is_none() {
        println!("Provide GPS device as first argument");
        std::process::exit(1);
    }

    let name = name.unwrap();

    match File::open(name) {
        Ok(socket) => return socket,
        Err(e)     => {
            println!("Error {}", e);
            std::process::exit(1);
        }
    };
}

fn parse_loop(mut nmea: &mut nmea::Nmea, socket: File) {
    let mut input = BufReader::new(socket);

    loop {
        let mut buffer = String::new();

        let size = match input.read_line(&mut buffer) {
            Ok(size) => size,
            Err(_)   => continue,
        };

        if size == 0 {
            break;
        }

        parse(&mut nmea, &buffer);
    }
}

fn parse(nmea: &mut nmea::Nmea, buffer: &String) {
    match nmea.parse(&buffer) {
        Ok(sentence) => {
            match sentence {
                nmea::SentenceType::GGA => display_fix(&nmea),
                nmea::SentenceType::GSA => display_precision(&nmea),
                _ => ()
            }
        },
        Err(error) => println!("E: {}", error),
    }
}

fn display_fix(nmea: &nmea::Nmea) {
    let date = nmea.fix_date
        .map(|d| format!("{}", d.format("%Y-%m-%d")))
        .unwrap_or_else(|| "none".to_string());

    let time = nmea.fix_time
        .map(|t| format!("{}", t.format("%H:%M:%S")))
        .unwrap_or_else(|| "none".to_string());

    println!("time: {}T{}Z", date, time);

    let lat = nmea.latitude
        .map(|l| format!("{:10.6}°", l))
        .unwrap_or_else(|| "None".to_string());

    let lon = nmea.longitude
        .map(|l| format!("{:>11.6}°", l))
        .unwrap_or_else(|| "None".to_string());

    let alt = nmea.altitude
        .map(|a| format!("{:>6.1}m", a))
        .unwrap_or_else(|| "None".to_string());

    println!("lat: {} lon: {} alt: {}", lat, lon, alt);
}

fn display_precision(nmea: &nmea::Nmea) {
    let hdop = nmea.hdop
        .map(|l| format!("{:5.2}", l))
        .unwrap_or_else(|| "None".to_string());

    let vdop = nmea.vdop
        .map(|l| format!("{:5.2}", l))
        .unwrap_or_else(|| "None".to_string());

    let pdop = nmea.pdop
        .map(|l| format!("{:5.2}", l))
        .unwrap_or_else(|| "None".to_string());

    let fix_sats = nmea.fix_satellites()
        .map(|l| format!("{:2}", l))
        .unwrap_or_else(|| "None".to_string());

    println!("hdop: {} vdop: {} pdop: {} fix sats: {}", hdop, vdop, pdop, fix_sats);
}
