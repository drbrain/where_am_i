use chrono::naive::NaiveTime;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::number::complete::*;
use nom::sequence::*;
use nom::IResult;

#[derive(Clone, Debug, PartialEq)]
pub enum NMEA {
    DTM(DTMdata),
    GAQ(GAQdata),
    GBQ(GBQdata),
    GBS(GBSdata),
    GGA(GGAdata),
    GLL(GLLdata),
    GLQ,
    GNQ,
    GNS,
    GPQ,
    GRS,
    GSA,
    GST,
    GSV,
    RMC,
    TXT,
    VLW,
    VTG,
    ZDA,
}

fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    let (input, matched) = take_while(|c| c != ',')(input)?;

    Ok((input, matched.to_string()))
}

fn checksum<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    let (input, checksum) = preceded(star, hex_digit1)(input)?;

    Ok((input, checksum.parse().unwrap()))
}

fn comma<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(",")(input)
}

fn dollar<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("$")(input)
}

fn dot<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(".")(input)
}

fn eol<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("\r\n")(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum EastWest {
    East,
    West,
}

fn east_west<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, EastWest, E> {
    let (input, ew) = alt((char('E'), char('W')))(input)?;

    let ew = match ew {
        'E' => EastWest::East,
        'W' => EastWest::West,
        _ => panic!("Unhandled alternate {:?}", ew),
    };

    Ok((input, ew))
}

fn flt32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map_res(recognize_float, |s: &str| s.parse())(input)
}

fn int32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i32, E> {
    map_res(
        recognize(preceded(opt(char('-')), take_while(is_digit))),
        |s: &str| s.parse(),
    )(input)
}

fn is_digit(chr: char) -> bool {
    chr.is_ascii_digit()
}

fn is_upper_alphanum(chr: char) -> bool {
    chr.is_ascii_uppercase() || chr.is_ascii_digit()
}

#[derive(Clone, Debug, PartialEq)]
pub enum NorthSouth {
    North,
    South,
}

fn lat<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map(tuple((two_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

fn lon<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map(tuple((three_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatLon {
    pub latitude: f32,
    pub longitude: f32,
}

fn latlon<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, LatLon, E> {
    let (input, latitude) = map(
        tuple((terminated(lat, comma), terminated(north_south, comma))),
        |(l, d)| l * if d == NorthSouth::North { 1.0 } else { -1.0 },
    )(input)?;

    let (input, longitude) = map(tuple((terminated(lon, comma), east_west)), |(l, d)| {
        l * if d == EastWest::East { 1.0 } else { -1.0 }
    })(input)?;

    Ok((
        input,
        LatLon {
            latitude,
            longitude,
        },
    ))
}

fn north_south<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NorthSouth, E> {
    let (input, ns) = alt((char('N'), char('S')))(input)?;

    let ns = match ns {
        'N' => NorthSouth::North,
        'S' => NorthSouth::South,
        _ => panic!("Unhandled alternate {:?}", ns),
    };

    Ok((input, ns))
}

#[derive(Clone, Debug, PartialEq)]
pub enum PositionMode {
    AutonomousGNSSFix,
    DifferentialGNSSFix,
    EstimatedDeadReckoningFix,
    NoFix,
    RTKFixed,
    RTKFloat,
}

fn pos_mode<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, PositionMode, E> {
    map(one_of("ADEFNR"), |c| match c {
        'A' => PositionMode::AutonomousGNSSFix,
        'D' => PositionMode::DifferentialGNSSFix,
        'E' => PositionMode::EstimatedDeadReckoningFix,
        'F' => PositionMode::RTKFloat,
        'N' => PositionMode::NoFix,
        'R' => PositionMode::RTKFixed,
        _ => panic!("Unhandled position mode {:?}", c),
    })(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Quality {
    AutonomousGNSSFix,
    DifferentialGNSSFix,
    EstimatedDeadReckoningFix,
    NoFix,
    RTKFixed,
    RTKFloat,
}

fn quality<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Quality, E> {
    map(one_of("012456"), |c| match c {
        '0' => Quality::NoFix,
        '1' => Quality::AutonomousGNSSFix,
        '2' => Quality::DifferentialGNSSFix,
        '4' => Quality::RTKFixed,
        '5' => Quality::RTKFloat,
        '6' => Quality::EstimatedDeadReckoningFix,
        _ => panic!("Unhandled quality {:?}", c),
    })(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Signal {
    GPSL1CA,
    GPSL2CL,
    GPSL2CM,
    GalileoE1C,
    GalileoE1B,
    GalileoE5bI,
    GalileoE5bQ,
    BeiDuoB1ID1,
    BeiDuoB1ID2,
    BeiDuoB2ID1,
    BeiDuoB2ID2,
    QZSSL1CA,
    QZSSL2CM,
    QZSSL2CL,
    GLONASSL1OF,
    GLONASSL2OF,
    Unknown,
}

fn signal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Signal, E> {
    let (input, signal_id) = uint32(input)?;

    let signal = match signal_id {
        1 => Signal::GPSL1CA,
        2 => Signal::GLONASSL1OF,
        3 => Signal::GalileoE1C,
        4 => Signal::BeiDuoB1ID1,
        _ => Signal::Unknown,
    };

    Ok((input, signal))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Valid,
    Invalid,
}

fn status<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Status, E> {
    map(one_of("AV"), |c| match c {
        'A' => Status::Valid,
        'V' => Status::Invalid,
        _ => panic!("Unhandled quality {:?}", c),
    })(input)
}

fn system<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Signal, E> {
    let (input, system_id) = uint32(input)?;

    let system = match system_id {
        1 => Signal::GPSL1CA,
        2 => Signal::GalileoE5bI,
        3 => Signal::BeiDuoB1ID1,
        5 => Signal::GPSL2CM,
        6 => Signal::GPSL2CL,
        7 => Signal::GalileoE1C,
        _ => Signal::Unknown,
    };

    Ok((input, system))
}

fn star<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("*")(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Talker {
    BeiDuo,
    Combination,
    ECDIS,
    GLONASS,
    GPS,
    Galileo,
    Unknown(String),
}

fn talker<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Talker, E> {
    let (input, talker) = take_while_m_n(2, 2, is_upper_alphanum)(input)?;

    let talker = match talker {
        "EI" => Talker::ECDIS,
        "GA" => Talker::Galileo,
        "GB" => Talker::BeiDuo,
        "GL" => Talker::GLONASS,
        "GN" => Talker::Combination,
        "GP" => Talker::GPS,
        _ => Talker::Unknown(talker.to_string()),
    };

    Ok((input, talker))
}

fn three_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    let (input, integer) = take_while_m_n(3, 3, is_digit)(input)?;

    let integer = integer.parse().unwrap();

    Ok((input, integer))
}

fn time<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NaiveTime, E> {
    let (input, (hour, minute, second, subsec)) =
        tuple((two_digit, two_digit, two_digit, preceded(dot, two_digit)))(input)?;

    let time = NaiveTime::from_hms_milli(hour, minute, second, subsec * 100);

    Ok((input, time))
}

fn two_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    let (input, integer) = take_while_m_n(2, 2, is_digit)(input)?;

    let integer = integer.parse().unwrap();

    Ok((input, integer))
}

fn uint32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while(is_digit), |s: &str| s.parse())(input)
}

fn verify_checksum(data: &str, checksum: &str) -> bool {
    let checksum = u8::from_str_radix(checksum, 16).unwrap();

    checksum == data.bytes().fold(0, |cs, b| cs ^ b)
}

fn line<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    let (input, line) = preceded(dollar, terminated(take_while1(|c| c != '\r'), eol))(input)?;

    let (_, (nmea_line, _, checksum)) = tuple((take_while1(|c| c != '*'), star, hex_digit1))(line)?;

    verify(rest, |_: &str| verify_checksum(nmea_line, checksum))("")?;

    Ok((input, nmea_line))
}

#[derive(Clone, Debug, PartialEq)]
pub struct DTMdata {
    pub talker: Talker,
    pub datum: String,
    pub sub_datum: String,
    pub lat: f32,
    pub north_south: NorthSouth,
    pub lon: f32,
    pub east_west: EastWest,
    pub alt: f32,
    pub ref_datum: String,
}

fn dtm<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, DTMdata, E> {
    let (_, (talker, datum, sub_datum, lat, north_south, lon, east_west, alt, ref_datum)) =
        tuple((
            terminated(talker, terminated(tag("DTM"), comma)),
            terminated(any, comma),
            terminated(any, comma),
            terminated(flt32, comma),
            terminated(north_south, comma),
            terminated(flt32, comma),
            terminated(east_west, comma),
            terminated(flt32, comma),
            any,
        ))(input)?;

    let data = DTMdata {
        talker,
        datum,
        sub_datum,
        lat,
        north_south,
        lon,
        east_west,
        alt,
        ref_datum,
    };

    Ok((input, data))
}

#[derive(Clone, Debug, PartialEq)]
pub struct GAQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn gaq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GAQdata, E> {
    let (_, (talker, message_id)) =
        tuple((talker, preceded(tag("GAQ"), preceded(comma, any))))(input)?;

    let data = GAQdata { talker, message_id };

    Ok((input, data))
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn gbq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBQdata, E> {
    let (_, (talker, message_id)) =
        tuple((talker, preceded(tag("GBQ"), preceded(comma, any))))(input)?;

    let data = GBQdata { talker, message_id };

    Ok((input, data))
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBSdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub err_lat: f32,
    pub err_lon: f32,
    pub err_alt: f32,
    pub svid: Option<u32>,
    pub prob: Option<f32>,
    pub bias: Option<f32>,
    pub stddev: Option<f32>,
    pub system: Option<Signal>,
    pub signal: Option<Signal>,
}

fn gbs<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBSdata, E> {
    let (
        input,
        (talker, time, err_lat, err_lon, err_alt, svid, prob, bias, stddev, system, signal),
    ) = tuple((
        terminated(talker, terminated(tag("GBS"), comma)),
        terminated(time, comma),
        terminated(flt32, comma),
        terminated(flt32, comma),
        terminated(flt32, comma),
        terminated(opt(uint32), comma),
        terminated(opt(flt32), comma),
        terminated(opt(flt32), comma),
        terminated(opt(flt32), comma),
        terminated(opt(system), comma),
        opt(signal),
    ))(input)?;

    let data = GBSdata {
        talker,
        time,
        err_lat,
        err_lon,
        err_alt,
        svid,
        prob,
        bias,
        stddev,
        system,
        signal,
    };

    Ok((input, data))
}

#[derive(Clone, Debug, PartialEq)]
pub struct GGAdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub lat_lon: LatLon,
    pub quality: Quality,
    pub num_satellites: u32,
    pub hdop: f32,
    pub alt: f32,
    pub alt_unit: String,
    pub sep: f32,
    pub sep_unit: String,
    pub diff_age: Option<u32>,
    pub diff_station: Option<u32>,
}

fn gga<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GGAdata, E> {
    let (
        input,
        (
            talker,
            time,
            lat_lon,
            quality,
            num_satellites,
            hdop,
            alt,
            alt_unit,
            sep,
            sep_unit,
            diff_age,
            diff_station,
        ),
    ) = tuple((
        terminated(talker, terminated(tag("GGA"), comma)),
        terminated(time, comma),
        terminated(latlon, comma),
        terminated(quality, comma),
        terminated(uint32, comma),
        terminated(flt32, comma),
        terminated(flt32, comma),
        terminated(any, comma),
        terminated(flt32, comma),
        terminated(any, comma),
        terminated(opt(uint32), comma),
        opt(uint32),
    ))(input)?;

    let data = GGAdata {
        talker,
        time,
        lat_lon,
        quality,
        num_satellites,
        hdop,
        alt,
        alt_unit,
        sep,
        sep_unit,
        diff_age,
        diff_station,
    };

    Ok((input, data))
}

#[derive(Clone, Debug, PartialEq)]
pub struct GLLdata {
    pub talker: Talker,
    pub lat_lon: LatLon,
    pub time: NaiveTime,
    pub status: Status,
    pub position_mode: PositionMode,
}

fn gll<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GLLdata, E> {
    let (input, (talker, lat_lon, time, status, position_mode)) = tuple((
        terminated(talker, terminated(tag("GLL"), comma)),
        terminated(latlon, comma),
        terminated(time, comma),
        terminated(status, comma),
        pos_mode,
    ))(input)?;

    let data = GLLdata {
        talker,
        lat_lon,
        time,
        status,
        position_mode,
    };

    Ok((input, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    type VE<'a> = VerboseError<&'a str>;

    #[test]
    fn test_comma() {
        assert_eq!(",", comma::<VE>(",").unwrap().1);
    }

    #[test]
    fn test_dollar() {
        assert_eq!("$", dollar::<VE>("$").unwrap().1);
    }

    #[test]
    fn test_lat() {
        assert_eq!(47.28521118, lat::<VE>("4717.112671").unwrap().1);
    }

    #[test]
    fn test_latlon() {
        let lat_lon = latlon::<VE>("4717.11399,N,00833.91590,W").unwrap().1;

        assert_eq!(47.285233, lat_lon.latitude);
        assert_eq!(-8.565265, lat_lon.longitude);
    }

    #[test]
    fn test_lon() {
        assert_eq!(8.56524738, lon::<VE>("00833.914843").unwrap().1);
    }

    #[test]
    fn test_line() {
        let full_line = "$GPDTM,W84,,0.0,N,0.0,E,0.0,W84*6F\r\n";

        assert_eq!(
            "GPDTM,W84,,0.0,N,0.0,E,0.0,W84",
            line::<VE>(full_line).unwrap().1
        );
    }

    #[test]
    fn test_pos_mode() {
        assert_eq!(PositionMode::NoFix, pos_mode::<VE>("N").unwrap().1);
        assert_eq!(
            PositionMode::AutonomousGNSSFix,
            pos_mode::<VE>("A").unwrap().1
        );
    }

    #[test]
    fn test_quality() {
        assert_eq!(Quality::NoFix, quality::<VE>("0").unwrap().1);
        assert_eq!(Quality::AutonomousGNSSFix, quality::<VE>("1").unwrap().1);
    }

    #[test]
    fn test_status() {
        assert_eq!(Status::Valid, status::<VE>("A").unwrap().1);
        assert_eq!(Status::Invalid, status::<VE>("V").unwrap().1);
    }

    #[test]
    fn test_talker() {
        assert_eq!(Talker::Galileo, talker::<VE>("GA").unwrap().1);
        assert_eq!(Talker::BeiDuo, talker::<VE>("GB").unwrap().1);
        assert_eq!(Talker::GLONASS, talker::<VE>("GL").unwrap().1);
        assert_eq!(Talker::Combination, talker::<VE>("GN").unwrap().1);
        assert_eq!(Talker::GPS, talker::<VE>("GP").unwrap().1);
        assert_eq!(
            Talker::Unknown("AA".to_string()),
            talker::<VE>("AA").unwrap().1
        );
    }

    #[test]
    fn test_dtm() {
        let parsed = dtm::<VE>("GPDTM,W84,,0.0,N,0.0,E,0.0,W84").unwrap().1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_eq!("W84".to_string(), parsed.datum);
        assert_eq!("".to_string(), parsed.sub_datum);
        assert_approx_eq!(0.0, parsed.lat);
        assert_eq!(NorthSouth::North, parsed.north_south);
        assert_approx_eq!(0.0, parsed.lon);
        assert_eq!(EastWest::East, parsed.east_west);
        assert_approx_eq!(0.0, parsed.alt);
        assert_eq!("W84".to_string(), parsed.ref_datum);

        let parsed = dtm::<VE>("GPDTM,999,,0.08,N,0.07,E,-47.7,W84").unwrap().1;

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
        let parsed = gaq::<VE>("EIGAQ,RMC").unwrap().1;

        assert_eq!(Talker::ECDIS, parsed.talker);
        assert_eq!("RMC".to_string(), parsed.message_id);
    }

    #[test]
    fn test_gbq() {
        let parsed = gbq::<VE>("EIGBQ,RMC").unwrap().1;

        assert_eq!(Talker::ECDIS, parsed.talker);
        assert_eq!("RMC".to_string(), parsed.message_id);
    }

    #[test]
    fn test_gbs() {
        let parsed = gbs::<VE>("GPGBS,235503.00,1.6,1.4,3.2,,,,,,").unwrap().1;

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

        let parsed = gbs::<VE>("GPGBS,235458.00,1.4,1.3,3.1,03,,-21.4,3.8,1,0")
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
        assert_eq!(Some(Signal::GPSL1CA), parsed.system);
        assert_eq!(Some(Signal::Unknown), parsed.signal);
    }

    #[test]
    fn test_gga() {
        let parsed =
            gga::<VE>("GPGGA,092725.00,4717.11399,N,00833.91590,E,1,08,1.01,499.6,M,48.0,M,,")
                .unwrap()
                .1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_eq!(NaiveTime::from_hms_milli(09, 27, 25, 0), parsed.time);
        assert_approx_eq!(47.285233, parsed.lat_lon.latitude);
        assert_approx_eq!(8.565265, parsed.lat_lon.longitude);
        assert_eq!(Quality::AutonomousGNSSFix, parsed.quality);
        assert_eq!(8, parsed.num_satellites);
        assert_approx_eq!(1.01, parsed.hdop);
        assert_approx_eq!(499.6, parsed.alt);
        assert_eq!("M".to_string(), parsed.alt_unit);
        assert_approx_eq!(48.0, parsed.sep);
        assert_eq!(None, parsed.diff_age);
        assert_eq!(None, parsed.diff_station);
    }

    #[test]
    fn test_gll() {
        let parsed = gll::<VE>("GPGLL,4717.11364,N,00833.91565,E,092321.00,A,A")
            .unwrap()
            .1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_approx_eq!(47.28523, parsed.lat_lon.latitude);
        assert_approx_eq!(8.565261, parsed.lat_lon.longitude);
        assert_eq!(NaiveTime::from_hms_milli(09, 23, 21, 0), parsed.time);
        assert_eq!(Status::Valid, parsed.status);
        assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
    }
}
