use chrono::naive::NaiveDate;
use chrono::naive::NaiveTime;

use core::num::NonZeroUsize;

use crate::gps::Driver;
use crate::gps::Generic;
use crate::nmea::parser;
use crate::nmea::parser::*;
use crate::nmea::EastWest;
use crate::nmea::NorthSouth;

use nom::error::VerboseErrorKind::Context;
use nom::error::*;
use nom::Err;
use nom::Needed;

use std::time::Duration;

type VE<'a> = VerboseError<&'a str>;
type VEb<'a> = VerboseError<&'a [u8]>;

fn p<'a, D>(input: &'a str, result: nom::IResult<&'a str, D, VE>) -> D {
    match result {
        Ok((_, data)) => data,
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            panic!("{}", convert_error(input, e));
        }
        Err(nom::Err::Incomplete(_)) => panic!("impossible incomplete error"),
    }
}

fn driver() -> Driver {
    Driver::Generic(Generic::default())
}

fn parse<'a>(input: &'a [u8]) -> NMEA {
    let driver = driver();

    parser::parse::<VEb>(input, &driver, timestamp()).unwrap().1
}

#[test]
fn test_parse() {
    let parsed = parse(b"$EIGAQ,RMC*2B\r\n$");
    let mut data = parser::gaq::<VE>("EIGAQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GAQ(data), parsed);
}

#[test]
fn test_parse_invalid_utf8() {
    let (input, error) = parser::parse::<VEb>(
        b"\x01\x1E$PUBX,40,ZDA,0,1,0,0,0,0*45\r\x1A\x18\x0F\x1F\x0C\xFF\xFF\xFF\xFF\xFF",
        &driver(),
        timestamp(),
    )
    .unwrap();

    assert_eq!(b"\xFF\xFF\xFF\xFF", input);
    assert_eq!(NMEA::ParseError(String::from("Invalid UTF-8")), error);
}

#[test]
fn test_unknown() {
    let parsed = parse(b"$GPROT,35.6,A*01\r\n");
    let data = "GPROT,35.6,A".to_string();

    assert_eq!(NMEA::Unsupported(data), parsed);
}

#[test]
fn test_error_checksum() {
    let result = parse(b"$EIGAQ,RMC*2C\r\n");

    let mismatch = ChecksumMismatch {
        message: String::from("EIGAQ,RMC"),
        given: 44,
        calculated: 43,
    };

    assert_eq!(NMEA::InvalidChecksum(mismatch), result);
}

#[test]
fn test_incomplete() {
    let input = b"$EIG";
    let result = parser::parse::<VEb>(input, &driver(), timestamp());

    if let Err(Err::Incomplete(e)) = result {
        assert_eq!(Needed::Size(NonZeroUsize::new(1).unwrap()), e);
    } else {
        assert!(false, "Was complete")
    }
}

#[test]
fn test_skip_garbage() {
    let parsed = parse(b"stuff*AA\r\n$EIGAQ,RMC*2B\r\n$");
    let mut data = parser::gaq::<VE>("EIGAQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GAQ(data), parsed);
}

#[test]
fn test_garbage() {
    let input = b"$";
    let (input, count) = parser::garbage::<VEb>(input).unwrap();

    assert_eq!(0, count);
    assert_eq!(b"$", input);

    let input = b"x$";
    let (input, count) = parser::garbage::<VEb>(input).unwrap();

    assert_eq!(1, count);
    assert_eq!(b"$", input);

    let input = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx$";
    let (input, count) = parser::garbage::<VEb>(input).unwrap();

    assert_eq!(164, count);
    assert_eq!(b"$", input);

    let input = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx$";
    let result = parser::garbage::<VEb>(input);

    if let Err(Err::Failure(mut f)) = result {
        assert_eq!(Context("garbage"), f.errors.pop().unwrap().1);
    } else {
        assert!(false, "Garbage limit not reached");
    }
}

#[test]
fn test_nmea_message() {
    let parsed = parser::nmea_message::<VE>("EIGAQ,RMC", timestamp())
        .unwrap()
        .1;
    let mut data = parser::gaq::<VE>("EIGAQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GAQ(data), parsed);

    let parsed = parser::nmea_message::<VE>("EIGNQ,RMC", timestamp())
        .unwrap()
        .1;
    let mut data = parser::gnq::<VE>("EIGNQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GNQ(data), parsed);
}

#[test]
fn test_nav_mode() {
    assert_eq!(
        NavigationMode::FixNone,
        parser::nav_mode::<VE>("1").unwrap().1
    );
    assert_eq!(
        NavigationMode::Fix2D,
        parser::nav_mode::<VE>("2").unwrap().1
    );
}

#[test]
fn test_pos_mode() {
    assert_eq!(PositionMode::NoFix, parser::pos_mode::<VE>("N").unwrap().1);
    assert_eq!(
        PositionMode::AutonomousGNSSFix,
        parser::pos_mode::<VE>("A").unwrap().1
    );
}

#[test]
fn test_quality() {
    assert_eq!(Quality::NoFix, parser::quality::<VE>("0").unwrap().1);
    assert_eq!(
        Quality::AutonomousGNSSFix,
        parser::quality::<VE>("1").unwrap().1
    );
}

#[test]
fn test_status() {
    assert_eq!(Status::Valid, parser::status::<VE>("A").unwrap().1);
    assert_eq!(Status::Invalid, parser::status::<VE>("V").unwrap().1);
}

#[test]
fn test_talker() {
    assert_eq!(Talker::Galileo, parser::talker::<VE>("GA").unwrap().1);
    assert_eq!(Talker::BeiDuo, parser::talker::<VE>("GB").unwrap().1);
    assert_eq!(Talker::GLONASS, parser::talker::<VE>("GL").unwrap().1);
    assert_eq!(Talker::Combination, parser::talker::<VE>("GN").unwrap().1);
    assert_eq!(Talker::GPS, parser::talker::<VE>("GP").unwrap().1);
    assert_eq!(
        Talker::Unknown("AA".to_string()),
        parser::talker::<VE>("AA").unwrap().1
    );
}

#[test]
fn test_dtm() {
    let parsed = parser::dtm::<VE>("GPDTM,W84,,0.0,N,0.0,E,0.0,W84")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!("W84".to_string(), parsed.datum);
    assert_eq!("".to_string(), parsed.sub_datum);
    assert_approx_eq!(0.0, parsed.lat);
    assert_eq!(NorthSouth::North, parsed.north_south);
    assert_approx_eq!(0.0, parsed.lon);
    assert_eq!(EastWest::East, parsed.east_west);
    assert_approx_eq!(0.0, parsed.alt);
    assert_eq!("W84".to_string(), parsed.ref_datum);

    let parsed = parser::dtm::<VE>("GPDTM,999,,0.08,N,0.07,E,-47.7,W84")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!("999".to_string(), parsed.datum);
    assert_eq!("".to_string(), parsed.sub_datum);
    assert_approx_eq!(0.08, parsed.lat);
    assert_eq!(NorthSouth::North, parsed.north_south);
    assert_approx_eq!(0.07, parsed.lon);
    assert_eq!(EastWest::East, parsed.east_west);
    assert_approx_eq!(-47.7, parsed.alt);
    assert_eq!("W84".to_string(), parsed.ref_datum);
}

#[test]
fn test_gaq() {
    let parsed = parser::gaq::<VE>("EIGAQ,RMC").unwrap().1;

    assert_eq!(Talker::ECDIS, parsed.talker);
    assert_eq!("RMC".to_string(), parsed.message_id);
}

#[test]
fn test_gbq() {
    let parsed = parser::gbq::<VE>("EIGBQ,RMC").unwrap().1;

    assert_eq!(Talker::ECDIS, parsed.talker);
    assert_eq!("RMC".to_string(), parsed.message_id);
}

#[test]
fn test_gbs() {
    let parsed = parser::gbs::<VE>("GPGBS,235503.00,1.6,1.4,3.2,,,,,,")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(23, 55, 3, 0), parsed.time);
    assert_approx_eq!(1.6, parsed.err_lat);
    assert_approx_eq!(1.4, parsed.err_lon);
    assert_approx_eq!(3.2, parsed.err_alt);
    assert_eq!(None, parsed.svid);
    assert_eq!(None, parsed.prob);
    assert_eq!(None, parsed.bias);
    assert_eq!(None, parsed.stddev);
    assert_eq!(None, parsed.stddev);
    assert_eq!(None, parsed.system);
    assert_eq!(None, parsed.signal);

    let parsed = parser::gbs::<VE>("GPGBS,235458.00,1.4,1.3,3.1,03,,-21.4,3.8,1,0")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(23, 54, 58, 0), parsed.time);
    assert_approx_eq!(1.4, parsed.err_lat);
    assert_approx_eq!(1.3, parsed.err_lon);
    assert_approx_eq!(3.1, parsed.err_alt);
    assert_eq!(Some(3), parsed.svid);
    assert_eq!(None, parsed.prob);
    assert_eq!(Some(-21.4), parsed.bias);
    assert_eq!(Some(3.8), parsed.stddev);
    assert_eq!(Some(System::GPS), parsed.system);
    assert_eq!(Some(Signal::Unknown), parsed.signal);
}

#[test]
fn test_gga() {
    let parsed =
        parser::gga::<VE>("GPGGA,092725.00,4717.11399,N,00833.91590,E,1,08,1.01,499.6,M,48.0,M,,")
            .unwrap()
            .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(09, 27, 25, 0), parsed.time);

    let lat_lon = parsed.lat_lon.unwrap();
    assert_approx_eq!(47.285233, lat_lon.latitude);
    assert_approx_eq!(8.565265, lat_lon.longitude);

    assert_eq!(Quality::AutonomousGNSSFix, parsed.quality);
    assert_eq!(8, parsed.num_satellites);
    assert_approx_eq!(1.01, parsed.hdop.unwrap());
    assert_approx_eq!(499.6, parsed.alt.unwrap());
    assert_eq!("M".to_string(), parsed.alt_unit);
    assert_approx_eq!(48.0, parsed.sep.unwrap());
    assert_eq!(None, parsed.diff_age);
    assert_eq!(None, parsed.diff_station);
}

#[test]
fn test_gga_startup() {
    let parsed = parser::gga::<VE>("GPGGA,204849.013,,,,,0,0,,,M,,M,,")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(20, 48, 49, 013), parsed.time);
    assert_eq!(None, parsed.lat_lon);
    assert_eq!(Quality::NoFix, parsed.quality);
    assert_eq!(0, parsed.num_satellites);
    assert_eq!(None, parsed.hdop);
    assert_eq!(None, parsed.alt);
    assert_eq!("M".to_string(), parsed.alt_unit);
    assert_eq!(None, parsed.sep);
    assert_eq!(None, parsed.diff_age);
    assert_eq!(None, parsed.diff_station);
}

#[test]
fn test_gll() {
    let parsed = parser::gll::<VE>("GPGLL,4717.11364,N,00833.91565,E,092321.00,A,A")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);

    let lat_lon = parsed.lat_lon.unwrap();
    assert_approx_eq!(47.28523, lat_lon.latitude);
    assert_approx_eq!(8.565261, lat_lon.longitude);

    assert_eq!(NaiveTime::from_hms_milli(09, 23, 21, 0), parsed.time);
    assert_eq!(Status::Valid, parsed.status);
    assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
}

#[test]
fn test_gll_startup() {
    let parsed = parser::gll::<VE>("GPGLL,,,,,204849.013,V,N").unwrap().1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(None, parsed.lat_lon);
    assert_eq!(NaiveTime::from_hms_milli(20, 48, 49, 013), parsed.time);
    assert_eq!(Status::Invalid, parsed.status);
    assert_eq!(PositionMode::NoFix, parsed.position_mode);
}

#[test]
fn test_glq() {
    let parsed = parser::glq::<VE>("EIGLQ,RMC").unwrap().1;

    assert_eq!(Talker::ECDIS, parsed.talker);
    assert_eq!("RMC".to_string(), parsed.message_id);
}

#[test]
fn test_gnq() {
    let parsed = parser::gnq::<VE>("EIGNQ,RMC").unwrap().1;

    assert_eq!(Talker::ECDIS, parsed.talker);
    assert_eq!("RMC".to_string(), parsed.message_id);
}

#[test]
fn test_gpq() {
    let parsed = parser::gpq::<VE>("EIGPQ,RMC").unwrap().1;

    assert_eq!(Talker::ECDIS, parsed.talker);
    assert_eq!("RMC".to_string(), parsed.message_id);
}

#[test]
fn test_grs() {
    let parsed = parser::grs::<VE>("GNGRS,104148.00,1,2.6,2.2,-1.6,-1.1,-1.7,-1.5,5.8,1.7,,,,,1,1")
        .unwrap()
        .1;

    let residuals = vec![
        Some(2.6),
        Some(2.2),
        Some(-1.6),
        Some(-1.1),
        Some(-1.7),
        Some(-1.5),
        Some(5.8),
        Some(1.7),
        None,
        None,
        None,
        None,
    ];

    assert_eq!(Talker::Combination, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(10, 41, 48, 0), parsed.time);
    assert_eq!(true, parsed.gga_includes_residuals);
    assert_eq!(residuals[0], parsed.residuals[0]);
    assert_eq!(residuals[1], parsed.residuals[1]);
    assert_eq!(residuals[2], parsed.residuals[2]);
    assert_eq!(residuals[3], parsed.residuals[3]);
    assert_eq!(residuals[4], parsed.residuals[4]);
    assert_eq!(residuals[5], parsed.residuals[5]);
    assert_eq!(residuals[6], parsed.residuals[6]);
    assert_eq!(residuals[7], parsed.residuals[7]);
    assert_eq!(residuals[8], parsed.residuals[8]);
    assert_eq!(residuals[9], parsed.residuals[9]);
    assert_eq!(residuals[10], parsed.residuals[10]);
    assert_eq!(residuals[11], parsed.residuals[11]);
    assert_eq!(System::GPS, parsed.system);
    assert_eq!(Some(Signal::L1), parsed.signal);

    let parsed = parser::grs::<VE>("GNGRS,104148.00,1,,0.0,2.5,0.0,,2.8,,,,,,,1,5")
        .unwrap()
        .1;

    let residuals = vec![
        None,
        Some(0.0),
        Some(2.5),
        Some(0.0),
        None,
        Some(2.8),
        None,
        None,
        None,
        None,
        None,
        None,
    ];

    assert_eq!(Talker::Combination, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(10, 41, 48, 0), parsed.time);
    assert_eq!(true, parsed.gga_includes_residuals);
    assert_eq!(residuals[0], parsed.residuals[0]);
    assert_eq!(residuals[1], parsed.residuals[1]);
    assert_eq!(residuals[2], parsed.residuals[2]);
    assert_eq!(residuals[3], parsed.residuals[3]);
    assert_eq!(residuals[4], parsed.residuals[4]);
    assert_eq!(residuals[5], parsed.residuals[5]);
    assert_eq!(residuals[6], parsed.residuals[6]);
    assert_eq!(residuals[7], parsed.residuals[7]);
    assert_eq!(residuals[8], parsed.residuals[8]);
    assert_eq!(residuals[9], parsed.residuals[9]);
    assert_eq!(residuals[10], parsed.residuals[10]);
    assert_eq!(residuals[11], parsed.residuals[11]);
    assert_eq!(System::GPS, parsed.system);
    assert_eq!(Some(Signal::L2CM), parsed.signal);
}

#[test]
fn test_gsa() {
    let parsed = parser::gsa::<VE>("GPGSA,A,3,23,29,07,08,09,18,26,28,,,,,1.94,1.18,1.54,1")
        .unwrap()
        .1;

    let satellite_ids = vec![
        Some(23),
        Some(29),
        Some(7),
        Some(8),
        Some(9),
        Some(18),
        Some(26),
        Some(28),
        None,
        None,
        None,
        None,
    ];

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(OperationMode::Automatic, parsed.operation_mode);
    assert_eq!(NavigationMode::Fix3D, parsed.navigation_mode);
    assert_eq!(satellite_ids[0], parsed.satellite_ids[0]);
    assert_eq!(satellite_ids[1], parsed.satellite_ids[1]);
    assert_eq!(satellite_ids[2], parsed.satellite_ids[2]);
    assert_eq!(satellite_ids[3], parsed.satellite_ids[3]);
    assert_eq!(satellite_ids[4], parsed.satellite_ids[4]);
    assert_eq!(satellite_ids[5], parsed.satellite_ids[5]);
    assert_eq!(satellite_ids[6], parsed.satellite_ids[6]);
    assert_eq!(satellite_ids[7], parsed.satellite_ids[7]);
    assert_eq!(satellite_ids[8], parsed.satellite_ids[8]);
    assert_eq!(satellite_ids[9], parsed.satellite_ids[9]);
    assert_eq!(satellite_ids[10], parsed.satellite_ids[10]);
    assert_eq!(satellite_ids[11], parsed.satellite_ids[11]);
    assert_approx_eq!(1.94, parsed.pdop.unwrap());
    assert_approx_eq!(1.18, parsed.hdop.unwrap());
    assert_approx_eq!(1.54, parsed.vdop.unwrap());
    assert_eq!(Some(System::GPS), parsed.system);
}

#[test]
fn test_gsa_startup() {
    let parsed = parser::gsa::<VE>("GPGSA,A,1,,,,,,,,,,,,,,,").unwrap().1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(OperationMode::Automatic, parsed.operation_mode);
    assert_eq!(NavigationMode::FixNone, parsed.navigation_mode);
    assert_eq!(None, parsed.satellite_ids[0]);
    assert_eq!(None, parsed.satellite_ids[1]);
    assert_eq!(None, parsed.satellite_ids[2]);
    assert_eq!(None, parsed.satellite_ids[3]);
    assert_eq!(None, parsed.satellite_ids[4]);
    assert_eq!(None, parsed.satellite_ids[5]);
    assert_eq!(None, parsed.satellite_ids[6]);
    assert_eq!(None, parsed.satellite_ids[7]);
    assert_eq!(None, parsed.satellite_ids[8]);
    assert_eq!(None, parsed.satellite_ids[9]);
    assert_eq!(None, parsed.satellite_ids[10]);
    assert_eq!(None, parsed.satellite_ids[11]);
    assert_eq!(None, parsed.pdop);
    assert_eq!(None, parsed.hdop);
    assert_eq!(None, parsed.vdop);
}

#[test]
fn test_gst() {
    let parsed = parser::gst::<VE>("GPGST,082356.00,1.8,,,,1.7,1.3,2.2")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(8, 23, 56, 0), parsed.time);
    assert_approx_eq!(1.8, parsed.range_rms.unwrap());
    assert_eq!(None, parsed.std_major);
    assert_eq!(None, parsed.std_minor);
    assert_eq!(None, parsed.orientation);
    assert_approx_eq!(1.7, parsed.std_lat.unwrap());
    assert_approx_eq!(1.3, parsed.std_lon.unwrap());
    assert_approx_eq!(2.2, parsed.std_alt.unwrap());
}

#[test]
fn test_gsv() {
    let (_, parsed) = parser::gsv::<VE>("GPGSV,3,1,09,09,,,17,10,,,40,12,,,49,13,,,35,1").unwrap();

    let satellites = vec![
        GSVsatellite {
            id: 9,
            elevation: None,
            azimuth: None,
            cno: Some(17),
        },
        GSVsatellite {
            id: 10,
            elevation: None,
            azimuth: None,
            cno: Some(40),
        },
        GSVsatellite {
            id: 12,
            elevation: None,
            azimuth: None,
            cno: Some(49),
        },
        GSVsatellite {
            id: 13,
            elevation: None,
            azimuth: None,
            cno: Some(35),
        },
    ];

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(3, parsed.num_msgs);
    assert_eq!(1, parsed.msg);
    assert_eq!(9, parsed.num_satellites);
    assert_eq!(satellites, parsed.satellites);
    assert_eq!(Some(Signal::L1), parsed.signal);

    let parsed = parser::gsv::<VE>("GPGSV,3,3,09,25,,,40,1").unwrap().1;

    let satellites = vec![GSVsatellite {
        id: 25,
        elevation: None,
        azimuth: None,
        cno: Some(40),
    }];

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(3, parsed.num_msgs);
    assert_eq!(3, parsed.msg);
    assert_eq!(9, parsed.num_satellites);
    assert_eq!(satellites, parsed.satellites);
    assert_eq!(Some(Signal::L1), parsed.signal);

    let parsed = parser::gsv::<VE>("GPGSV,1,1,03,12,,,42,24,,,47,32,,,37,5")
        .unwrap()
        .1;

    let satellites = vec![
        GSVsatellite {
            id: 12,
            elevation: None,
            azimuth: None,
            cno: Some(42),
        },
        GSVsatellite {
            id: 24,
            elevation: None,
            azimuth: None,
            cno: Some(47),
        },
        GSVsatellite {
            id: 32,
            elevation: None,
            azimuth: None,
            cno: Some(37),
        },
    ];

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(1, parsed.num_msgs);
    assert_eq!(1, parsed.msg);
    assert_eq!(3, parsed.num_satellites);
    assert_eq!(satellites, parsed.satellites);
    assert_eq!(Some(Signal::L2CM), parsed.signal);

    let parsed = parser::gsv::<VE>("GAGSV,1,1,00,2").unwrap().1;

    let satellites: Vec<GSVsatellite> = vec![];

    assert_eq!(Talker::Galileo, parsed.talker);
    assert_eq!(1, parsed.num_msgs);
    assert_eq!(1, parsed.msg);
    assert_eq!(0, parsed.num_satellites);
    assert_eq!(satellites, parsed.satellites);
    assert_eq!(Some(Signal::E5), parsed.signal);
}

#[test]
fn test_gsv_startup() {
    let (_, parsed) = parser::gsv::<VE>("GPGSV,1,1,00").unwrap();

    let satellites: Vec<GSVsatellite> = vec![];

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(1, parsed.num_msgs);
    assert_eq!(1, parsed.msg);
    assert_eq!(0, parsed.num_satellites);
    assert_eq!(satellites, parsed.satellites);
    assert_eq!(None, parsed.signal);
}

#[test]
fn test_gbgsv() {
    let input = "GBGSV,2,1,07,04,00,261,,11,01,341,,12,30,300,,19,61,071,,";
    let result = gsv::<VE>(input);

    let parsed = p::<GSVData>(input, result);

    let satellites = vec![
        GSVsatellite {
            id: 4,
            elevation: Some(0),
            azimuth: Some(261),
            cno: None,
        },
        GSVsatellite {
            id: 11,
            elevation: Some(1),
            azimuth: Some(341),
            cno: None,
        },
        GSVsatellite {
            id: 12,
            elevation: Some(30),
            azimuth: Some(300),
            cno: None,
        },
        GSVsatellite {
            id: 19,
            elevation: Some(61),
            azimuth: Some(71),
            cno: None,
        },
    ];

    assert_eq!(Talker::BeiDuo, parsed.talker);
    assert_eq!(2, parsed.num_msgs);
    assert_eq!(1, parsed.msg);
    assert_eq!(7, parsed.num_satellites);
    assert_eq!(satellites, parsed.satellites);
    assert_eq!(None, parsed.signal);
}

#[test]
fn test_rmc() {
    let parsed =
        parser::rmc::<VE>("GPRMC,083559.00,A,4717.11437,N,00833.91522,E,0.004,77.52,091202,,,A,V")
            .unwrap()
            .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(08, 35, 59, 0), parsed.time);
    assert_eq!(Status::Valid, parsed.status);

    let lat_lon = parsed.lat_lon.unwrap();
    assert_approx_eq!(47.28524, lat_lon.latitude);
    assert_approx_eq!(8.565253, lat_lon.longitude);

    assert_approx_eq!(0.004, parsed.speed);
    assert_approx_eq!(77.52, parsed.course_over_ground.unwrap());
    assert_eq!(NaiveDate::from_ymd(02, 12, 9), parsed.date);
    assert_eq!(None, parsed.magnetic_variation);
    assert_eq!(None, parsed.magnetic_variation_east_west);
    assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
    assert_eq!(Some(Status::Invalid), parsed.nav_status);
}

#[test]
fn test_rmc_empty() {
    let parsed = parser::rmc::<VE>("GPRMC,204849.013,V,,,,,0.00,0.00,050920,,,N")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(20, 48, 49, 013), parsed.time);
    assert_eq!(Status::Invalid, parsed.status);

    assert_eq!(None, parsed.lat_lon);

    assert_approx_eq!(0.0, parsed.speed);
    assert_approx_eq!(0.0, parsed.course_over_ground.unwrap());
    assert_eq!(NaiveDate::from_ymd(20, 9, 5), parsed.date);
    assert_eq!(None, parsed.magnetic_variation);
    assert_eq!(None, parsed.magnetic_variation_east_west);
    assert_eq!(PositionMode::NoFix, parsed.position_mode);
    assert_eq!(None, parsed.nav_status);
}

#[test]
fn test_gnrmc() {
    let input = "GNRMC,083559.00,A,4717.11437,N,00833.91522,E,0.015,,091202,,,A,V";

    let result = rmc::<VE>(input);

    let parsed = p::<RMCData>(input, result);

    assert_eq!(Talker::Combination, parsed.talker);
    assert_eq!(NaiveTime::from_hms_milli(08, 35, 59, 0), parsed.time);
    assert_eq!(Status::Valid, parsed.status);

    let lat_lon = parsed.lat_lon.unwrap();
    assert_approx_eq!(47.28524, lat_lon.latitude);
    assert_approx_eq!(8.565253, lat_lon.longitude);

    assert_approx_eq!(0.015, parsed.speed);
    assert_eq!(None, parsed.course_over_ground);
    assert_eq!(NaiveDate::from_ymd(02, 12, 9), parsed.date);
    assert_eq!(None, parsed.magnetic_variation);
    assert_eq!(None, parsed.magnetic_variation_east_west);
    assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
    assert_eq!(Some(Status::Invalid), parsed.nav_status);
}

#[test]
fn test_txt() {
    let parsed = parser::txt::<VE>("GPTXT,01,01,02,u-blox ag - www.u-blox.com")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(1, parsed.num_msgs);
    assert_eq!(1, parsed.msg);
    assert_eq!(MessageType::Notice, parsed.msg_type);
    assert_eq!("u-blox ag - www.u-blox.com".to_string(), parsed.text);
}

#[test]
fn test_vlw() {
    let parsed = parser::vlw::<VE>("GPVLW,,N,,N,15.8,N,1.2,N").unwrap().1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_eq!(None, parsed.total_water_distance);
    assert_eq!("N", parsed.total_water_distance_unit);
    assert_eq!(None, parsed.water_distance);
    assert_eq!("N", parsed.water_distance_unit);
    assert_approx_eq!(15.8, parsed.total_ground_distance);
    assert_eq!("N", parsed.total_ground_distance_unit);
    assert_approx_eq!(1.2, parsed.ground_distance);
    assert_eq!("N", parsed.ground_distance_unit);
}

#[test]
fn test_vtg() {
    let parsed = parser::vtg::<VE>("GPVTG,77.52,T,,M,0.004,N,0.008,K,A")
        .unwrap()
        .1;

    assert_eq!(Talker::GPS, parsed.talker);
    assert_approx_eq!(77.52, parsed.course_over_ground_true.unwrap());
    assert_eq!("T", parsed.course_over_ground_true_unit);
    assert_eq!(None, parsed.course_over_ground_magnetic);
    assert_eq!("M", parsed.course_over_ground_magnetic_unit);
    assert_approx_eq!(0.004, parsed.speed_over_ground_knots);
    assert_eq!("N", parsed.speed_over_ground_knots_unit);
    assert_approx_eq!(0.008, parsed.speed_over_ground_km);
    assert_eq!("K", parsed.speed_over_ground_km_unit);
    assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
}

#[test]
fn test_gnvtg() {
    let input = "GNVTG,,T,,M,0.015,N,0.027,K,A";

    let result = vtg::<VE>(input);

    let parsed = p::<VTGData>(input, result);

    assert_eq!(Talker::Combination, parsed.talker);
    assert_eq!(None, parsed.course_over_ground_true);
    assert_eq!("T", parsed.course_over_ground_true_unit);
    assert_eq!(None, parsed.course_over_ground_magnetic);
    assert_eq!("M", parsed.course_over_ground_magnetic_unit);
    assert_approx_eq!(0.015, parsed.speed_over_ground_knots);
    assert_eq!("N", parsed.speed_over_ground_knots_unit);
    assert_approx_eq!(0.027, parsed.speed_over_ground_km);
    assert_eq!("K", parsed.speed_over_ground_km_unit);
    assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
}

#[test]
fn test_zda() {
    let parsed = parser::zda::<VE>("GPZDA,082710.00,16,09,2002,00,00")
        .unwrap()
        .1;

    assert_eq!(Some(NaiveTime::from_hms_milli(8, 27, 10, 0)), parsed.time);
    assert_eq!(Some(16), parsed.day);
    assert_eq!(Some(9), parsed.month);
    assert_eq!(Some(2002), parsed.year);
    assert_eq!(0, parsed.local_tz_hour);
    assert_eq!(0, parsed.local_tz_minute);
}

#[test]
fn test_zda_empty() {
    let parsed = parser::zda::<VE>("GPZDA,,,,,00,00").unwrap().1;

    assert_eq!(None, parsed.time);
    assert_eq!(None, parsed.day);
    assert_eq!(None, parsed.month);
    assert_eq!(None, parsed.year);
    assert_eq!(0, parsed.local_tz_hour);
    assert_eq!(0, parsed.local_tz_minute);
}

#[test]
fn test_zda_time_only() {
    let parsed = parser::zda::<VE>("GPZDA,233346.00,,,,00,00").unwrap().1;

    assert_eq!(Some(NaiveTime::from_hms_milli(23, 33, 46, 0)), parsed.time);
    assert_eq!(None, parsed.day);
    assert_eq!(None, parsed.month);
    assert_eq!(None, parsed.year);
    assert_eq!(0, parsed.local_tz_hour);
    assert_eq!(0, parsed.local_tz_minute);
}

fn timestamp() -> Duration {
    Duration::from_secs(7)
}
