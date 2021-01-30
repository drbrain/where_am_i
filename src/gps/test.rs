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
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let gga = GGAData {
        received: None,
        talker: Talker::GPS,
        time: NaiveTime::from_hms_milli(1, 8, 2, 0),
        lat_lon: Some(LatLon {
            latitude: 44.9343,
            longitude: -93.2624,
        }),
        quality: Quality::AutonomousGNSSFix,
        num_satellites: 12,
        hdop: Some(1.0),
        alt: Some(264.0),
        alt_unit: "m".to_string(),
        sep: Some(0.0),
        sep_unit: "M".to_string(),
        diff_age: None,
        diff_station: None,
    };

    gps.gga(gga, "name", &gpsd_tx, &ntp_tx);

    let lat_lon = gps.lat_lon.unwrap();

    assert_eq!(Quality::AutonomousGNSSFix, gps.quality.unwrap());
    assert_approx_eq!(44.9343, lat_lon.latitude);
    assert_approx_eq!(-93.2624, lat_lon.longitude);
    assert_approx_eq!(264.0, gps.altitude_msl.unwrap());
}

#[test]
fn test_gsa() {
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let gbgsa = GSAData {
        received: None,
        talker: Talker::BeiDuo,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::BeiDuo),
    };

    let gagsa = GSAData {
        received: None,
        talker: Talker::Galileo,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::Galileo),
    };

    let glgsa = GSAData {
        received: None,
        talker: Talker::GLONASS,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::GLONASS),
    };

    let gpgsa = GSAData {
        received: None,
        talker: Talker::GPS,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::GPS),
    };

    gps.gsa(gagsa, "name", &gpsd_tx, &ntp_tx);
    gps.gsa(gbgsa, "name", &gpsd_tx, &ntp_tx);
    gps.gsa(glgsa, "name", &gpsd_tx, &ntp_tx);
    gps.gsa(gpgsa, "name", &gpsd_tx, &ntp_tx);

    assert_eq!(None, gps.beiduo_navigation_mode);
    assert_eq!(None, gps.galileo_navigation_mode);
    assert_eq!(None, gps.glonass_navigation_mode);
    assert_eq!(None, gps.gps_navigation_mode);
}

#[test]
fn test_gsa_beiduo() {
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let gbgsa = GSAData {
        received: None,
        talker: Talker::BeiDuo,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::BeiDuo),
    };

    gps.gsa(gbgsa, "name", &gpsd_tx, &ntp_tx);
    assert_eq!(NavigationMode::Fix3D, gps.beiduo_navigation_mode.unwrap());
}

#[test]
fn test_gsa_galileo() {
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let gagsa = GSAData {
        received: None,
        talker: Talker::Galileo,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::Galileo),
    };

    gps.gsa(gagsa, "name", &gpsd_tx, &ntp_tx);
    assert_eq!(NavigationMode::Fix3D, gps.galileo_navigation_mode.unwrap());
}

#[test]
fn test_gsa_glonass() {
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let glgsa = GSAData {
        received: None,
        talker: Talker::GLONASS,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::GLONASS),
    };

    gps.gsa(glgsa, "name", &gpsd_tx, &ntp_tx);
    assert_eq!(NavigationMode::Fix3D, gps.glonass_navigation_mode.unwrap());
}

#[test]
fn test_gsa_gps() {
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let gpgsa = GSAData {
        received: None,
        talker: Talker::GPS,
        operation_mode: OperationMode::Automatic,
        navigation_mode: NavigationMode::Fix3D,
        satellite_ids: vec![Some(1), Some(2), Some(3)],
        pdop: Some(1.0),
        hdop: Some(1.0),
        vdop: Some(1.0),
        system: Some(System::GPS),
    };

    gps.gsa(gpgsa, "name", &gpsd_tx, &ntp_tx);
    assert_eq!(NavigationMode::Fix3D, gps.gps_navigation_mode.unwrap());
}

#[test]
fn test_zda() {
    let (gpsd_tx, _) = broadcast::channel(1);
    let (ntp_tx, _) = broadcast::channel(1);
    let mut gps = GPSData::default();

    let zda = ZDAData {
        received: None,
        talker: Talker::GPS,
        time: Some(NaiveTime::from_hms_milli(1, 8, 0, 0)),
        day: Some(26),
        month: Some(5),
        year: Some(2020),
        local_tz_hour: 0,
        local_tz_minute: 0,
    };

    let expected_time = build_time(2020, 5, 26, 1, 8, 0, 0);

    gps.zda(zda, "name", &gpsd_tx, &ntp_tx);

    assert_eq!(2020, gps.year);
    assert_eq!(expected_time, gps.time.unwrap());
}
