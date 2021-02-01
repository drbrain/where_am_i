#[cfg(test)]
mod test {
    use crate::gps::ublox_nmea::ubx_00;
    use crate::gps::ublox_nmea::ubx_03;
    use crate::gps::ublox_nmea::ubx_04;
    use crate::gps::*;

    use chrono::NaiveDate;
    use chrono::NaiveTime;

    use nom::error::*;

    type VE<'a> = VerboseError<&'a str>;

    fn p<'a, D>(input: &'a str, result: nom::IResult<&'a str, D, VE>) -> D {
        match result {
            Ok((_, data)) => data,
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                panic!("{}", convert_error(input, e));
            }
            Err(nom::Err::Incomplete(_)) => panic!("impossible incomplete error"),
        }
    }

    #[test]
    fn test_ubx_00() {
        let input = "PUBX,00,081350.00,4717.113210,N,00833.915187,E,546.589,G3,2.1,2.0,0.007,77.52,0.007,,0.92,1.19,0.77,9,0,0";
        let result = ubx_00::<VE>(input);

        let parsed = p::<UBXPosition>(input, result);

        assert_eq!(NaiveTime::from_hms_milli(8, 13, 50, 0), parsed.time);

        let lat_lon = parsed.lat_lon.unwrap();
        assert_approx_eq!(47.28522, lat_lon.latitude);
        assert_approx_eq!(8.565253, lat_lon.longitude);

        assert_approx_eq!(546.589, parsed.alt_ref);
        assert_eq!(UBXNavigationStatus::Standalone3D, parsed.nav_status);
        assert_approx_eq!(2.1, parsed.horizontal_accuracy);
        assert_approx_eq!(2.0, parsed.vertical_accuracy);
        assert_approx_eq!(0.007, parsed.speed_over_ground);
        assert_approx_eq!(77.52, parsed.course_over_ground);
        assert_approx_eq!(0.007, parsed.vertical_velocity);
        assert_eq!(None, parsed.diff_age);
        assert_approx_eq!(0.92, parsed.hdop);
        assert_approx_eq!(1.19, parsed.vdop);
        assert_approx_eq!(0.77, parsed.tdop);
        assert_eq!(9, parsed.num_satellites);
        assert_eq!(0, parsed.reserved);
        assert_eq!(false, parsed.dead_reckoning);
    }

    #[test]
    fn test_ubx_03() {
        let input = "PUBX,03,11,23,-,,,45,010,29,-,,,46,013,07,-,,,42,015,08,U,067,31,42,025,10,U,195,33,46,026,18,U,326,08,39,026,17,-,,,32,015,26,U,306,66,48,025,27,U,073,10,36,026,28,U,089,61,46,024,15,-,,,39,014";
        let result = ubx_03::<VE>(input);

        let parsed = p::<UBXSatellites>(input, result);

        let satellites = vec![
            UBXSatellite {
                id: 23,
                status: UBXSatelliteStatus::NotUsed,
                azimuth: None,
                elevation: None,
                cno: 45,
                lock_time: 10,
            },
            UBXSatellite {
                id: 29,
                status: UBXSatelliteStatus::NotUsed,
                azimuth: None,
                elevation: None,
                cno: 46,
                lock_time: 13,
            },
            UBXSatellite {
                id: 7,
                status: UBXSatelliteStatus::NotUsed,
                azimuth: None,
                elevation: None,
                cno: 42,
                lock_time: 15,
            },
            UBXSatellite {
                id: 8,
                status: UBXSatelliteStatus::Used,
                azimuth: Some(67),
                elevation: Some(31),
                cno: 42,
                lock_time: 25,
            },
            UBXSatellite {
                id: 10,
                status: UBXSatelliteStatus::Used,
                azimuth: Some(195),
                elevation: Some(33),
                cno: 46,
                lock_time: 26,
            },
            UBXSatellite {
                id: 18,
                status: UBXSatelliteStatus::Used,
                azimuth: Some(326),
                elevation: Some(8),
                cno: 39,
                lock_time: 26,
            },
            UBXSatellite {
                id: 17,
                status: UBXSatelliteStatus::NotUsed,
                azimuth: None,
                elevation: None,
                cno: 32,
                lock_time: 15,
            },
            UBXSatellite {
                id: 26,
                status: UBXSatelliteStatus::Used,
                azimuth: Some(306),
                elevation: Some(66),
                cno: 48,
                lock_time: 25,
            },
            UBXSatellite {
                id: 27,
                status: UBXSatelliteStatus::Used,
                azimuth: Some(73),
                elevation: Some(10),
                cno: 36,
                lock_time: 26,
            },
            UBXSatellite {
                id: 28,
                status: UBXSatelliteStatus::Used,
                azimuth: Some(89),
                elevation: Some(61),
                cno: 46,
                lock_time: 24,
            },
            UBXSatellite {
                id: 15,
                status: UBXSatelliteStatus::NotUsed,
                azimuth: None,
                elevation: None,
                cno: 39,
                lock_time: 14,
            },
        ];

        assert_eq!(satellites, parsed.satellites);
    }

    #[test]
    fn test_ubx_04() {
        let input = "PUBX,04,073731.00,091202,113851.00,1196,15D,1930035,-2660.664,43,";
        let result = ubx_04::<VE>(input);

        let parsed = p::<UBXTime>(input, result);

        assert_eq!(NaiveTime::from_hms_milli(7, 37, 31, 0), parsed.time);
        assert_eq!(NaiveDate::from_ymd(2, 12, 9), parsed.date);
        assert_approx_eq!(113851.0, parsed.time_of_week);
        assert_eq!(1196, parsed.week);
        assert_eq!(15, parsed.leap_seconds);
        assert_eq!(true, parsed.leap_second_default);
        assert_eq!(1930035, parsed.clock_bias);
        assert_approx_eq!(-2660.664, parsed.clock_drift);
        assert_eq!(43, parsed.time_pulse_granularity);
    }
}
