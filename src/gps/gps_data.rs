use chrono::prelude::*;

use crate::nmea::*;
use crate::JsonSender;

use serde_json::json;

use std::time::SystemTime;

use tracing::error;

#[derive(Debug, Default)]
pub struct GPSData {
    pub(crate) naive_date: Option<NaiveDate>,
    pub(crate) naive_time: Option<NaiveTime>,
    pub(crate) year: i32,

    pub time: Option<DateTime<Utc>>,

    pub lat_lon: Option<LatLon>,
    pub altitude_msl: Option<f32>,

    pub gps_navigation_mode: Option<NavigationMode>,
    pub glonass_navigation_mode: Option<NavigationMode>,
    pub galileo_navigation_mode: Option<NavigationMode>,
    pub beiduo_navigation_mode: Option<NavigationMode>,
    mode: Option<u32>,

    pub quality: Option<Quality>,
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
            NMEA::GGA(nd) => self.gga(nd, name, tx),
            NMEA::GSA(nd) => self.gsa(nd, name, tx),
            NMEA::ZDA(nd) => self.zda(nd, name, tx),
            _ => (),
        }
    }

    pub(crate) fn update_time(&mut self, new_time: NaiveTime) {
        if let Some(mut date) = self.naive_date {
            if let Some(time) = self.naive_time {
                if new_time < time {
                    date = date.succ();
                }
            }

            let time = NaiveDateTime::new(date, new_time);

            self.time = Some(DateTime::from_utc(time, Utc));
        }
    }

    pub(crate) fn gga(&mut self, gga: GGAData, name: &str, tx: &JsonSender) {
        self.quality = Some(gga.quality);
        self.lat_lon = Some(gga.lat_lon);
        self.altitude_msl = Some(gga.alt);
    }

    pub(crate) fn gsa(&mut self, gsa: GSAData, name: &str, tx: &JsonSender) {
        match gsa.system {
            System::BeiDuo => self.beiduo_navigation_mode = Some(gsa.navigation_mode),
            System::GLONASS => self.glonass_navigation_mode = Some(gsa.navigation_mode),
            System::GPS => self.gps_navigation_mode = Some(gsa.navigation_mode),
            System::Galileo => self.galileo_navigation_mode = Some(gsa.navigation_mode),
            _ => return,
        }

        let mut modes = Vec::with_capacity(4);

        if let Some(beiduo) = &self.beiduo_navigation_mode {
            modes.push(beiduo);
        }

        if let Some(glonass) = &self.glonass_navigation_mode {
            modes.push(glonass);
        }

        if let Some(gps) = &self.gps_navigation_mode {
            modes.push(gps);
        }

        if let Some(galileo) = &self.galileo_navigation_mode {
            modes.push(galileo);
        }

        if modes.len() == 4 {
            self.mode = Some(modes.iter().map(|m| gpsd_mode(m)).fold(0, u32::max));

            self.beiduo_navigation_mode = None;
            self.galileo_navigation_mode = None;
            self.glonass_navigation_mode = None;
            self.gps_navigation_mode = None;
        }
    }

    pub(crate) fn zda(&mut self, zda: ZDAData, name: &str, tx: &JsonSender) {
        let date = NaiveDate::from_ymd(zda.year, zda.month, zda.day);
        let time = NaiveDateTime::new(date, zda.time);
        let time = DateTime::from_utc(time, Utc);

        self.time = Some(time);
        self.year = time.year();

        report_toff(time, name, tx);
        report_tpv(time, self.mode, name, tx);
    }
}

fn gpsd_mode(navigation_mode: &NavigationMode) -> u32 {
    match navigation_mode {
        NavigationMode::FixNone => 1,
        NavigationMode::Fix2D => 2,
        NavigationMode::Fix3D => 3,
    }
}

fn report_toff(date: DateTime<Utc>, name: &str, tx: &JsonSender) {
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

fn report_tpv(time: DateTime<Utc>, mode: Option<u32>, name: &str, tx: &JsonSender) {
    let time = time.to_rfc3339();
    let mode = mode.unwrap_or(0);

    let tpv = json!({
        "class":  "TPV".to_string(),
        "device": name,
        "time":   time,
        "mode":   mode,
    });

    if tx.send(tpv).is_ok() {}
}
