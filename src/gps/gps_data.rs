use crate::gpsd::Response;
use crate::gpsd::Toff;
use crate::gpsd::Tpv;
use crate::nmea::*;
use crate::TSSender;
use crate::Timestamp;
use chrono::prelude::*;
use std::fmt::Debug;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::broadcast;
use tracing::error;
use tracing::trace;

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
    pub fn read_nmea(
        &mut self,
        nmea: NMEA,
        name: &str,
        gpsd_tx: &broadcast::Sender<Response>,
        ntp_tx: &TSSender,
    ) {
        match nmea {
            NMEA::InvalidChecksum(cm) => error!(
                "checksum match, given {}, calculated {} on {}",
                cm.given, cm.calculated, cm.message
            ),
            NMEA::ParseError(e) => error!("parse error: {}", e),
            NMEA::ParseFailure(f) => error!("parse failure: {}", f),
            NMEA::Unsupported(n) => error!("unsupported: {}", n),
            NMEA::GGA(nd) => self.gga(nd, name, gpsd_tx, ntp_tx),
            NMEA::GSA(nd) => self.gsa(nd, name, gpsd_tx, ntp_tx),
            NMEA::RMC(nd) => self.rmc(nd, name, gpsd_tx, ntp_tx),
            NMEA::ZDA(nd) => self.zda(nd, name, gpsd_tx, ntp_tx),
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

            let utc_time = DateTime::from_utc(time, Utc);

            if let Some(stored) = self.time {
                if stored == utc_time {
                    return;
                }
            }

            trace!("Time updated to {}", utc_time.format("%Y-%m-%dT%H:%M:%SZ"));

            self.time = Some(utc_time);
        }
    }

    // updates lat_lon and time
    pub(crate) fn gga(
        &mut self,
        gga: GGAData,
        _name: &str,
        _gpsd_tx: &broadcast::Sender<Response>,
        _ntp_tx: &TSSender,
    ) {
        self.quality = Some(gga.quality);
        self.lat_lon = gga.lat_lon;
        self.altitude_msl = gga.alt;

        self.update_time(gga.time);
    }

    // updates navigation modes
    pub(crate) fn gsa(
        &mut self,
        gsa: GSAData,
        _name: &str,
        _gpsd_tx: &broadcast::Sender<Response>,
        _ntp_tx: &TSSender,
    ) {
        match gsa.system {
            Some(System::BeiDuo) => self.beiduo_navigation_mode = Some(gsa.navigation_mode),
            Some(System::GLONASS) => self.glonass_navigation_mode = Some(gsa.navigation_mode),
            Some(System::GPS) => self.gps_navigation_mode = Some(gsa.navigation_mode),
            Some(System::Galileo) => self.galileo_navigation_mode = Some(gsa.navigation_mode),
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

    // updates lat_lon and time
    pub(crate) fn rmc(
        &mut self,
        rmc: RMCData,
        _name: &str,
        _gpsd_tx: &broadcast::Sender<Response>,
        _ntp_tx: &TSSender,
    ) {
        self.lat_lon = rmc.lat_lon;

        let reference = NaiveDateTime::new(rmc.date, rmc.time);
        let reference = DateTime::from_utc(reference, Utc);

        self.time = Some(reference);
        self.year = reference.year();
    }

    // updates time and date
    pub(crate) fn zda(
        &mut self,
        zda: ZDAData,
        name: &str,
        gpsd_tx: &broadcast::Sender<Response>,
        ntp_tx: &TSSender,
    ) {
        let year = match zda.year {
            Some(y) => y,
            None => return,
        };

        let month = match zda.month {
            Some(m) => m,
            None => return,
        };

        let day = match zda.day {
            Some(d) => d,
            None => return,
        };

        let time = match zda.time {
            Some(t) => t,
            None => return,
        };

        let received = match zda.received {
            Some(d) => d,
            None => timestamp(),
        };

        let date = NaiveDate::from_ymd(year, month, day);
        let reference = NaiveDateTime::new(date, time);
        let reference = DateTime::from_utc(reference, Utc);

        self.time = Some(reference);
        self.year = reference.year();

        report_toff(reference, received, name, gpsd_tx);
        report_tpv(reference, self.mode, name, gpsd_tx);
        report_ntp(reference, received, name, ntp_tx);
    }
}

fn gpsd_mode(navigation_mode: &NavigationMode) -> u32 {
    match navigation_mode {
        NavigationMode::FixNone => 1,
        NavigationMode::Fix2D => 2,
        NavigationMode::Fix3D => 3,
    }
}

fn report_ntp(reference: DateTime<Utc>, received: Duration, _name: &str, tx: &TSSender) {
    let ts = Timestamp {
        received_sec: received.as_secs(),
        received_nsec: received.subsec_nanos(),
        reference_sec: reference.timestamp() as u64,
        reference_nsec: reference.timestamp_subsec_nanos(),
    };

    if tx.send(ts).is_ok() {};
}

fn report_toff(
    reference: DateTime<Utc>,
    received: Duration,
    name: &str,
    tx: &broadcast::Sender<Response>,
) {
    let toff = Response::Toff(Toff {
        device: name.to_string(),
        real_sec: reference.timestamp(),
        real_nsec: reference.timestamp_subsec_nanos(),
        clock_sec: received.as_secs(),
        clock_nsec: received.subsec_nanos(),
    });

    if tx.send(toff).is_ok() {}
}

fn report_tpv(
    reference: DateTime<Utc>,
    mode: Option<u32>,
    name: &str,
    tx: &broadcast::Sender<Response>,
) {
    let time = reference.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mode = mode.unwrap_or(0);

    let tpv = Response::Tpv(Tpv {
        device: name.to_string(),
        time,
        mode,
    });

    if tx.send(tpv).is_ok() {}
}

fn timestamp() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
}
