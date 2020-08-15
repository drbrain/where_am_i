use chrono::prelude::*;

use crate::gps::GPSData;
use crate::nmea::*;

use tokio::sync::broadcast;

fn build_time(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    milli: u32,
) -> DateTime<Utc> {
    let date = NaiveDate::from_ymd(year, month, day);
    let time = NaiveTime::from_hms_milli(hour, minute, second, milli);
    let time = NaiveDateTime::new(date, time);

    DateTime::from_utc(time, Utc)
}

#[test]
fn test_update_time() {
    let mut gps = GPSData::default();
    gps.naive_date = Some(NaiveDate::from_ymd(2020, 5, 26));
    gps.time = Some(build_time(2020, 5, 26, 1, 8, 0, 0));

    let new_time = NaiveTime::from_hms_milli(1, 8, 1, 0);

    gps.update_time(new_time);

    let expected = Some(build_time(2020, 5, 26, 1, 8, 1, 0));

    assert_eq!(expected, gps.time);
}

#[test]
fn test_update_time_day_boundary() {
    let mut gps = GPSData::default();
    gps.naive_date = Some(NaiveDate::from_ymd(2020, 5, 25));
    gps.naive_time = Some(NaiveTime::from_hms_milli(23, 59, 59, 0));
    gps.time = Some(build_time(2020, 5, 25, 23, 59, 59, 0));

    let new_time = NaiveTime::from_hms_milli(0, 0, 0, 0);

    gps.update_time(new_time);

    let expected = Some(build_time(2020, 5, 26, 0, 0, 0, 0));

    assert_eq!(expected, gps.time);
}

#[test]
fn test_gga() {
    let (tx, _) = broadcast::channel(1);

    let mut gps = GPSData::default();
    let gga = GGAData {
        talker: Talker::GPS,
        time: NaiveTime::from_hms_milli(1, 8, 2, 0),
        lat_lon: LatLon {
            latitude: 44.9343,
            longitude: -93.2624,
        },
        quality: Quality::AutonomousGNSSFix,
        num_satellites: 12,
        hdop: 1.0,
        alt: 264.0,
        alt_unit: "m".to_string(),
        sep: 0.0,
        sep_unit: "M".to_string(),
        diff_age: None,
        diff_station: None,
    };

    gps.gga(gga, "name", &tx);

    let lat_lon = gps.lat_lon.unwrap();

    assert_eq!(Quality::AutonomousGNSSFix, gps.quality);
    assert_approx_eq!(44.9343, lat_lon.latitude);
    assert_approx_eq!(-93.2624, lat_lon.latitude);
    assert_approx_eq!(264.0, gps.altitude_msl.unwrap());
}
