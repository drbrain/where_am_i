extern crate nmea;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let mut nmea = nmea::Nmea::new();

    let name = env::args().nth(1).unwrap();
    let socket = File::open(name).unwrap();

    let mut input = BufReader::new(socket);

    loop {
        let mut buffer = String::new();
        let size = match input.read_line(&mut buffer) {
            Ok(size) => size,
            Err(_)   => continue,
        };

        if size > 0 {
            match nmea.parse(&buffer) {
                Ok(sentence) => {
                    match sentence {
                        nmea::SentenceType::GGA => {
                            let date = nmea.fix_date
                                .map(|d| format!("{}", d.format("%Y-%m-%d")))
                                .unwrap_or_else(|| "none".to_string());

                            let time = nmea.fix_time
                                .map(|t| format!("{}", t.format("%H:%M:%S")))
                                .unwrap_or_else(|| "none".to_string());

                            println!("fix: {}T{}Z", date, time);

                            let lat = nmea.latitude
                                .map(|l| format!("{:?}", l))
                                .unwrap_or_else(|| "None".to_string());

                            let lon = nmea.longitude
                                .map(|l| format!("{:?}", l))
                                .unwrap_or_else(|| "None".to_string());

                            let alt = nmea.altitude
                                .map(|a| format!("{:?}", a))
                                .unwrap_or_else(|| "None".to_string());

                            println!("lat: {:>6.9}° lon: {:>6.10}° alt: {:>1.6}m", lat, lon, alt);
                        },
                        nmea::SentenceType::GSA => {
                            let hdop = nmea.hdop
                                .map(|l| format!("{:?}", l))
                                .unwrap_or_else(|| "None".to_string());

                            let vdop = nmea.vdop
                                .map(|l| format!("{:?}", l))
                                .unwrap_or_else(|| "None".to_string());

                            let pdop = nmea.pdop
                                .map(|l| format!("{:?}", l))
                                .unwrap_or_else(|| "None".to_string());

                            let fix_sats = nmea.fix_satellites()
                                .map(|l| format!("{:?}", l))
                                .unwrap_or_else(|| "None".to_string());

                            println!("hdop: {:>1.4} vdop: {:>1.4} pdop: {:>1.4} fix sats: {}", hdop, vdop, pdop, fix_sats);
                        },
                        _ => (),
                    }
                },
                Err(error) => println!("E: {}", error),
            }
        } else {
            break;
        }
    }
}

