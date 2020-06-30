use nmea::{Nmea, SentenceType};

use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

type NmeaMutex = Arc<Mutex<Nmea>>;
type NmeaParseResult = Result<SentenceType, String>;

fn main() {
    let socket = open_socket();

    let nmea = Nmea::new();
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

fn parse_loop(nmea_m: &NmeaMutex, socket: File) {
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

fn location_loop(nmea_m: &NmeaMutex) {
    loop {
        thread::sleep(Duration::from_secs(10));

        display_location(&nmea_m);
        display_precision(&nmea_m);
        display_satellites(&nmea_m);
    };
}

fn parse(nmea_m: &NmeaMutex, buffer: &String) {
    match parse_line(&nmea_m, &buffer) {
        Ok(sentence) => {
            match sentence {
                SentenceType::GGA => display_time(&nmea_m),
                _ => ()
            }
        },
        Err(error) => println!("E: {}", error),
    };
}

fn parse_line(nmea_m: &NmeaMutex, buffer: &String) -> NmeaParseResult {
    let mut nmea = nmea_m.lock().unwrap();

    let result = nmea.parse(&buffer);

    return result;
}

fn display_time(nmea_m: &NmeaMutex) {
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

fn display_location(nmea_m: &NmeaMutex) {
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

fn display_precision(nmea_m: &NmeaMutex) {
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

fn display_satellites(nmea_m: &NmeaMutex) {
    let nmea = nmea_m.lock().unwrap();
    let satellites = nmea.satellites();

    for satellite in satellites {
        let snr = satellite.snr()
            .map(|snr| format!("{:2}dB", snr))
            .unwrap_or_else(|| " ?dB".to_string());

        let azimuth = satellite.azimuth()
            .map(|snr| format!("Az: {:3}°", snr))
            .unwrap_or_else(|| "Untracked".to_string());

        let elevation = satellite.elevation()
            .map(|snr| format!("El: {:2}°", snr))
            .unwrap_or_else(|| "".to_string());

        println!("{:3} ({}) {} {} {}",
                 satellite.prn(), satellite.gnss_type(),
                 snr, azimuth, elevation);
    }
}
