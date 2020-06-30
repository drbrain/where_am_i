extern crate nmea;

use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

fn main() {
    let socket = open_socket();

    let nmea = nmea::Nmea::new();
    let nmea_m = Arc::new(Mutex::new(nmea));

    parse_loop(&nmea_m, socket);
    location_loop(&nmea_m);
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

fn parse_loop(nmea_m: &Arc<Mutex<nmea::Nmea>>, socket: File) {
    let nmea_m = Arc::clone(&nmea_m);

    thread::spawn(move || {
        let mut input = BufReader::new(socket);

        loop {
            let mut buffer = String::new();

            let size = match input.read_line(&mut buffer) {
                Ok(size) => size,
                Err(_)   => continue,
            };

            if size == 0 {
                std::process::exit(1);
            }

            parse(&nmea_m, &buffer);
        }
    });
}

fn location_loop(nmea_m: &Arc<Mutex<nmea::Nmea>>) {
    loop {
        thread::sleep(Duration::from_secs(1));

        display_location(&nmea_m);
        display_precision(&nmea_m);
    };
}

fn parse(nmea_m: &Arc<Mutex<nmea::Nmea>>, buffer: &String) {
    match parse_line(&nmea_m, &buffer) {
        Ok(sentence) => {
            match sentence {
                nmea::SentenceType::GGA => display_time(&nmea_m),
                _ => ()
            }
        },
        Err(error) => println!("E: {}", error),
    };
}

fn parse_line(nmea_m: &Arc<Mutex<nmea::Nmea>>, buffer: &String) -> Result<nmea::SentenceType, String> {
    let mut nmea = nmea_m.lock().unwrap();

    let result = nmea.parse(&buffer);

    return result;
}

fn display_time(nmea_m: &Arc<Mutex<nmea::Nmea>>) {
    let nmea = nmea_m.lock().unwrap();

    let date      = nmea.fix_date;
    let time      = nmea.fix_time;

    let date = date
        .map(|d| format!("{}", d.format("%Y-%m-%d")))
        .unwrap_or_else(|| "none".to_string());

    let time = time
        .map(|t| format!("{}", t.format("%H:%M:%S")))
        .unwrap_or_else(|| "none".to_string());

    println!("time: {}T{}Z", date, time);
}

fn display_location(nmea_m: &Arc<Mutex<nmea::Nmea>>) {
    let nmea = nmea_m.lock().unwrap();

    let latitude  = nmea.latitude;
    let longitude = nmea.longitude;
    let altitude  = nmea.altitude;

    let lat = latitude
        .map(|l| format!("{:10.6}°", l))
        .unwrap_or_else(|| "None".to_string());

    let lon = longitude
        .map(|l| format!("{:>11.6}°", l))
        .unwrap_or_else(|| "None".to_string());

    let alt = altitude
        .map(|a| format!("{:>6.1}m", a))
        .unwrap_or_else(|| "None".to_string());

    println!("lat: {} lon: {} alt: {}", lat, lon, alt);
}

fn display_precision(nmea_m: &Arc<Mutex<nmea::Nmea>>) {
    let nmea = nmea_m.lock().unwrap();
    let hdop = nmea.hdop;
    let pdop = nmea.pdop;
    let vdop = nmea.vdop;
    let sats = nmea.fix_satellites();

    let hdop = hdop
        .map(|l| format!("{:5.2}", l))
        .unwrap_or_else(|| "None".to_string());

    let vdop = vdop
        .map(|l| format!("{:5.2}", l))
        .unwrap_or_else(|| "None".to_string());

    let pdop = pdop
        .map(|l| format!("{:5.2}", l))
        .unwrap_or_else(|| "None".to_string());

    let fix_sats = sats
        .map(|l| format!("{:2}", l))
        .unwrap_or_else(|| "None".to_string());

    println!("hdop: {} vdop: {} pdop: {} fix sats: {}", hdop, vdop, pdop, fix_sats);
}
