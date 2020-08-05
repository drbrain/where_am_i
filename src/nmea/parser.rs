use chrono::naive::NaiveDate;
use chrono::naive::NaiveTime;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::number::complete::*;
use nom::sequence::*;
use nom::Err;
use nom::IResult;

#[derive(Clone, Debug, PartialEq)]
pub enum NMEA {
    DTM(DTMdata),
    GAQ(GAQdata),
    GBQ(GBQdata),
    GBS(GBSdata),
    GGA(GGAdata),
    GLL(GLLdata),
    GLQ(GLQdata),
    GNQ(GNQdata),
    GNS(GNSdata),
    GPQ(GPQdata),
    GRS(GRSdata),
    GSA(GSAdata),
    GST(GSTdata),
    GSV(GSVdata),
    RMC(RMCdata),
    TXT(TXTdata),
    VLW(VLWdata),
    VTG(VTGdata),
    ZDA(ZDAdata),
}

pub fn parse<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NMEA, E> {
    terminated(
        preceded(
            dollar,
            map(map_res(checksum_verify, |m| message::<E>(m)), |tuple| {
                tuple.1
            }),
        ),
        eol,
    )(input)
}

fn message<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NMEA, E> {
    alt((
        map(dtm, |m| NMEA::DTM(m)),
        map(gaq, |m| NMEA::GAQ(m)),
        map(gbq, |m| NMEA::GBQ(m)),
        map(gbs, |m| NMEA::GBS(m)),
        map(gga, |m| NMEA::GGA(m)),
        map(gll, |m| NMEA::GLL(m)),
        map(glq, |m| NMEA::GLQ(m)),
        map(gnq, |m| NMEA::GNQ(m)),
        map(gns, |m| NMEA::GNS(m)),
        map(gpq, |m| NMEA::GPQ(m)),
        map(grs, |m| NMEA::GRS(m)),
        map(gsa, |m| NMEA::GSA(m)),
        map(gst, |m| NMEA::GST(m)),
        map(gsv, |m| NMEA::GSV(m)),
        map(rmc, |m| NMEA::RMC(m)),
        map(txt, |m| NMEA::TXT(m)),
        map(vlw, |m| NMEA::VLW(m)),
        map(vtg, |m| NMEA::VTG(m)),
        map(zda, |m| NMEA::ZDA(m)),
    ))(input)
}

fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    map(take_while(|c| c != ','), |m: &str| m.to_string())(input)
}

fn checksum<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(preceded(star, hex_digit1), |c| c.parse())(input)
}

fn checksum_check(data: &str, expected: &str) -> bool {
    let expected = u8::from_str_radix(expected, 16).unwrap();

    expected == data.bytes().fold(0, |cs, b| cs ^ b)
}

fn checksum_verify<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    map(
        context(
            "checksum verification",
            cut(verify(
                tuple((terminated(take_while1(|c| c != '*'), star), hex_digit1)),
                |(message, expected)| checksum_check(message, expected),
            )),
        ),
        |tuple| tuple.0,
    )(input)
}

fn comma<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(",")(input)
}

fn date<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NaiveDate, E> {
    map(
        tuple((two_digit, two_digit, two_digit_i)),
        |(day, month, year)| NaiveDate::from_ymd(year, month, day),
    )(input)
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
    map(one_of("EW"), |ew| match ew {
        'E' => EastWest::East,
        'W' => EastWest::West,
        _ => panic!("Unknown direction {:?}", ew),
    })(input)
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
    map(
        tuple((
            map(
                tuple((terminated(lat, comma), terminated(north_south, comma))),
                |(l, d)| l * if d == NorthSouth::North { 1.0 } else { -1.0 },
            ),
            map(tuple((terminated(lon, comma), east_west)), |(l, d)| {
                l * if d == EastWest::East { 1.0 } else { -1.0 }
            }),
        )),
        |(latitude, longitude)| LatLon {
            latitude,
            longitude,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    Error,
    Notice,
    User,
    Warning,
    Unknown(u32),
}

fn msg_type<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, MessageType, E> {
    map(two_digit, |t| match t {
        0 => MessageType::Error,
        1 => MessageType::Warning,
        2 => MessageType::Notice,
        7 => MessageType::User,
        _ => MessageType::Unknown(t),
    })(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavigationMode {
    FixNone,
    Fix2D,
    Fix3D,
}

fn nav_mode<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NavigationMode, E> {
    map(one_of("123"), |c| match c {
        '1' => NavigationMode::FixNone,
        '2' => NavigationMode::Fix2D,
        '3' => NavigationMode::Fix3D,
        _ => panic!("Unhandled navigation mode {:?}", c),
    })(input)
}

fn north_south<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NorthSouth, E> {
    map(one_of("NS"), |ns| match ns {
        'N' => NorthSouth::North,
        'S' => NorthSouth::South,
        _ => panic!("Unhandled direction {:?}", ns),
    })(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum OperationMode {
    Automatic,
    Manual,
}

fn op_mode<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, OperationMode, E> {
    map(one_of("AM"), |c| match c {
        'A' => OperationMode::Automatic,
        'M' => OperationMode::Manual,
        _ => panic!("Unhandled operation mode {:?}", c),
    })(input)
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

#[derive(Clone, Debug, Eq, PartialEq)]
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
    map(uint32, |c| match c {
        1 => Signal::GPSL1CA,
        2 => Signal::GLONASSL1OF,
        3 => Signal::GalileoE1C,
        4 => Signal::BeiDuoB1ID1,
        _ => Signal::Unknown,
    })(input)
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
    map(one_of("123567"), |c| match c {
        '1' => Signal::GPSL1CA,
        '2' => Signal::GalileoE5bI,
        '3' => Signal::BeiDuoB1ID1,
        '5' => Signal::GPSL2CM,
        '6' => Signal::GPSL2CL,
        '7' => Signal::GalileoE1C,
        _ => Signal::Unknown,
    })(input)
}

fn star<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("*")(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    map(take_while_m_n(2, 2, is_upper_alphanum), |t| match t {
        "EI" => Talker::ECDIS,
        "GA" => Talker::Galileo,
        "GB" => Talker::BeiDuo,
        "GL" => Talker::GLONASS,
        "GN" => Talker::Combination,
        "GP" => Talker::GPS,
        _ => Talker::Unknown(t.to_string()),
    })(input)
}

fn three_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(3, 3, is_digit), |i: &str| i.parse())(input)
}

fn time<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NaiveTime, E> {
    map(
        tuple((two_digit, two_digit, two_digit, preceded(dot, two_digit))),
        |(hour, minute, second, subsec)| {
            NaiveTime::from_hms_milli(hour, minute, second, subsec * 100)
        },
    )(input)
}

fn two_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

fn two_digit_i<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i32, E> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

fn uint32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while(is_digit), |s: &str| s.parse())(input)
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
    context(
        "DTM",
        all_consuming(map(
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
            )),
            |(talker, datum, sub_datum, lat, north_south, lon, east_west, alt, ref_datum)| {
                DTMdata {
                    talker,
                    datum,
                    sub_datum,
                    lat,
                    north_south,
                    lon,
                    east_west,
                    alt,
                    ref_datum,
                }
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GAQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn gaq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GAQdata, E> {
    context(
        "GAQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GAQ"), preceded(comma, any)))),
            |(talker, message_id)| GAQdata { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn gbq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBQdata, E> {
    context(
        "GBQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GBQ"), preceded(comma, any)))),
            |(talker, message_id)| GBQdata { talker, message_id },
        )),
    )(input)
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
    context(
        "GBS",
        all_consuming(map(
            tuple((
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
            )),
            |(
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
            )| {
                GBSdata {
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
                }
            },
        )),
    )(input)
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
    context(
        "GGA",
        all_consuming(map(
            tuple((
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
            )),
            |(
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
            )| GGAdata {
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
            },
        )),
    )(input)
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
    context(
        "GLL",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("GLL"), comma)),
                terminated(latlon, comma),
                terminated(time, comma),
                terminated(status, comma),
                pos_mode,
            )),
            |(talker, lat_lon, time, status, position_mode)| GLLdata {
                talker,
                lat_lon,
                time,
                status,
                position_mode,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GLQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn glq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GLQdata, E> {
    context(
        "GLQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GLQ"), preceded(comma, any)))),
            |(talker, message_id)| GLQdata { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GNQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn gnq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GNQdata, E> {
    context(
        "GNQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GNQ"), preceded(comma, any)))),
            |(talker, message_id)| GNQdata { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GNSdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub lat_lon: LatLon,
    pub gps_position_mode: PositionMode,
    pub glonass_position_mode: PositionMode,
    pub galileo_position_mode: PositionMode,
    pub beiduo_position_mode: PositionMode,
    pub num_satellites: u32,
    pub hdop: f32,
    pub alt: f32,
    pub sep: f32,
    pub diff_age: Option<u32>,
    pub diff_station: Option<u32>,
    pub nav_status: Status,
}

fn gns<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GNSdata, E> {
    context(
        "GNS",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("GNS"), comma)),
                terminated(time, comma),
                terminated(latlon, comma),
                pos_mode,
                pos_mode,
                pos_mode,
                terminated(pos_mode, comma),
                terminated(uint32, comma),
                terminated(flt32, comma),
                terminated(flt32, comma),
                terminated(flt32, comma),
                terminated(opt(uint32), comma),
                terminated(opt(uint32), comma),
                status,
            )),
            |(
                talker,
                time,
                lat_lon,
                gps_position_mode,
                glonass_position_mode,
                galileo_position_mode,
                beiduo_position_mode,
                num_satellites,
                hdop,
                alt,
                sep,
                diff_age,
                diff_station,
                nav_status,
            )| GNSdata {
                talker,
                time,
                lat_lon,
                gps_position_mode,
                glonass_position_mode,
                galileo_position_mode,
                beiduo_position_mode,
                num_satellites,
                hdop,
                alt,
                sep,
                diff_age,
                diff_station,
                nav_status,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GPQdata {
    pub talker: Talker,
    pub message_id: String,
}

fn gpq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GPQdata, E> {
    context(
        "GPQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GPQ"), preceded(comma, any)))),
            |(talker, message_id)| GPQdata { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GRSdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub gga_includes_residuals: bool,
    pub residuals: Vec<Option<f32>>,
    pub system: Signal,
    pub signal: Signal,
}

fn grs<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GRSdata, E> {
    context(
        "GRS",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("GRS"), comma)),
                terminated(time, comma),
                terminated(map(one_of("10"), |c| c == '1'), comma),
                map(many_m_n(12, 12, terminated(opt(flt32), comma)), |rs| {
                    Vec::from(rs)
                }),
                terminated(system, comma),
                signal,
            )),
            |(talker, time, gga_includes_residuals, residuals, system, signal)| GRSdata {
                talker,
                time,
                gga_includes_residuals,
                residuals,
                system,
                signal,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GSAdata {
    pub talker: Talker,
    pub operation_mode: OperationMode,
    pub navigation_mode: NavigationMode,
    pub satellite_ids: Vec<Option<u32>>,
    pub pdop: f32,
    pub hdop: f32,
    pub vdop: f32,
    pub system: Signal,
}

fn gsa<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSAdata, E> {
    context(
        "GSA",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("GSA"), comma)),
                terminated(op_mode, comma),
                terminated(nav_mode, comma),
                map(many_m_n(12, 12, terminated(opt(uint32), comma)), |sids| {
                    Vec::from(sids)
                }),
                terminated(flt32, comma),
                terminated(flt32, comma),
                terminated(flt32, comma),
                system,
            )),
            |(talker, operation_mode, navigation_mode, satellite_ids, pdop, hdop, vdop, system)| {
                GSAdata {
                    talker,
                    operation_mode,
                    navigation_mode,
                    satellite_ids,
                    pdop,
                    hdop,
                    vdop,
                    system,
                }
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GSTdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub range_rms: Option<f32>,
    pub std_major: Option<f32>,
    pub std_minor: Option<f32>,
    pub orientation: Option<f32>,
    pub std_lat: Option<f32>,
    pub std_lon: Option<f32>,
    pub std_alt: Option<f32>,
}

fn gst<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSTdata, E> {
    context(
        "GST",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("GST"), comma)),
                terminated(time, comma),
                terminated(opt(flt32), comma),
                terminated(opt(flt32), comma),
                terminated(opt(flt32), comma),
                terminated(opt(flt32), comma),
                terminated(opt(flt32), comma),
                terminated(opt(flt32), comma),
                opt(flt32),
            )),
            |(
                talker,
                time,
                range_rms,
                std_major,
                std_minor,
                orientation,
                std_lat,
                std_lon,
                std_alt,
            )| GSTdata {
                talker,
                time,
                range_rms,
                std_major,
                std_minor,
                orientation,
                std_lat,
                std_lon,
                std_alt,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GSVsatellite {
    pub id: u32,
    pub elevation: Option<u32>,
    pub azimuth: Option<u32>,
    pub cno: Option<u32>,
}

fn gsv_sat<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSVsatellite, E> {
    map(
        tuple((
            terminated(uint32, comma),
            terminated(opt(uint32), comma),
            terminated(opt(uint32), comma),
            opt(uint32),
        )),
        |(id, elevation, azimuth, cno)| GSVsatellite {
            id,
            elevation,
            azimuth,
            cno,
        },
    )(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GSVdata {
    pub talker: Talker,
    pub num_msgs: u32,
    pub msg: u32,
    pub num_satellites: u32,
    pub satellites: Vec<GSVsatellite>,
    pub signal: Signal,
}

fn gsv<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSVdata, E> {
    context(
        "GSV",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("GSV"), comma)),
                terminated(uint32, comma),
                terminated(uint32, comma),
                terminated(uint32, comma),
                many_m_n(0, 4, terminated(gsv_sat, comma)),
                signal,
            )),
            |(talker, num_msgs, msg, num_satellites, satellites, signal)| GSVdata {
                talker,
                num_msgs,
                msg,
                num_satellites,
                satellites,
                signal,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RMCdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub status: Status,
    pub lat_lon: LatLon,
    pub speed: f32,
    pub course_over_ground: f32,
    pub date: NaiveDate,
    pub magnetic_variation: Option<f32>,
    pub magnetic_variation_east_west: Option<EastWest>,
    pub position_mode: PositionMode,
    pub nav_status: Status,
}

fn rmc<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, RMCdata, E> {
    context(
        "RMC",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("RMC"), comma)),
                terminated(time, comma),
                terminated(status, comma),
                terminated(latlon, comma),
                terminated(flt32, comma),
                terminated(flt32, comma),
                terminated(date, comma),
                terminated(opt(flt32), comma),
                terminated(opt(east_west), comma),
                terminated(pos_mode, comma),
                status,
            )),
            |(
                talker,
                time,
                status,
                lat_lon,
                speed,
                course_over_ground,
                date,
                magnetic_variation,
                magnetic_variation_east_west,
                position_mode,
                nav_status,
            )| RMCdata {
                talker,
                time,
                status,
                lat_lon,
                speed,
                course_over_ground,
                date,
                magnetic_variation,
                magnetic_variation_east_west,
                position_mode,
                nav_status,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TXTdata {
    pub talker: Talker,
    pub num_msgs: u32,
    pub msg: u32,
    pub msg_type: MessageType,
    pub text: String,
}

fn txt<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, TXTdata, E> {
    context(
        "TXT",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("TXT"), comma)),
                terminated(uint32, comma),
                terminated(uint32, comma),
                terminated(msg_type, comma),
                any,
            )),
            |(talker, num_msgs, msg, msg_type, text)| TXTdata {
                talker,
                num_msgs,
                msg,
                msg_type,
                text,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VLWdata {
    pub talker: Talker,
    pub total_water_distance: Option<f32>,
    pub total_water_distance_unit: String,
    pub water_distance: Option<f32>,
    pub water_distance_unit: String,
    pub total_ground_distance: f32,
    pub total_ground_distance_unit: String,
    pub ground_distance: f32,
    pub ground_distance_unit: String,
}

fn vlw<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VLWdata, E> {
    context(
        "VLW",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("VLW"), comma)),
                terminated(opt(flt32), comma),
                terminated(any, comma),
                terminated(opt(flt32), comma),
                terminated(any, comma),
                terminated(flt32, comma),
                terminated(any, comma),
                terminated(flt32, comma),
                any,
            )),
            |(
                talker,
                total_water_distance,
                total_water_distance_unit,
                water_distance,
                water_distance_unit,
                total_ground_distance,
                total_ground_distance_unit,
                ground_distance,
                ground_distance_unit,
            )| VLWdata {
                talker,
                total_water_distance,
                total_water_distance_unit,
                water_distance,
                water_distance_unit,
                total_ground_distance,
                total_ground_distance_unit,
                ground_distance,
                ground_distance_unit,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VTGdata {
    pub talker: Talker,
    pub course_over_ground_true: f32,
    pub course_over_ground_true_unit: String,
    pub course_over_ground_magnetic: Option<f32>,
    pub course_over_ground_magnetic_unit: String,
    pub speed_over_ground_knots: f32,
    pub speed_over_ground_knots_unit: String,
    pub speed_over_ground_km: f32,
    pub speed_over_ground_km_unit: String,
    pub position_mode: PositionMode,
}

fn vtg<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VTGdata, E> {
    context(
        "VTG",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("VTG"), comma)),
                terminated(flt32, comma),
                terminated(any, comma),
                terminated(opt(flt32), comma),
                terminated(any, comma),
                terminated(flt32, comma),
                terminated(any, comma),
                terminated(flt32, comma),
                terminated(any, comma),
                pos_mode,
            )),
            |(
                talker,
                course_over_ground_true,
                course_over_ground_true_unit,
                course_over_ground_magnetic,
                course_over_ground_magnetic_unit,
                speed_over_ground_knots,
                speed_over_ground_knots_unit,
                speed_over_ground_km,
                speed_over_ground_km_unit,
                position_mode,
            )| VTGdata {
                talker,
                course_over_ground_true,
                course_over_ground_true_unit,
                course_over_ground_magnetic,
                course_over_ground_magnetic_unit,
                speed_over_ground_knots,
                speed_over_ground_knots_unit,
                speed_over_ground_km,
                speed_over_ground_km_unit,
                position_mode,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZDAdata {
    pub talker: Talker,
    pub time: NaiveTime,
    pub day: u32,
    pub month: u32,
    pub year: i32,
    pub local_tz_hour: i32,
    pub local_tz_minute: u32,
}

fn zda<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, ZDAdata, E> {
    context(
        "ZDA",
        all_consuming(map(
            tuple((
                terminated(talker, terminated(tag("ZDA"), comma)),
                terminated(time, comma),
                terminated(uint32, comma),
                terminated(uint32, comma),
                terminated(int32, comma),
                terminated(int32, comma),
                uint32,
            )),
            |(talker, time, day, month, year, local_tz_hour, local_tz_minute)| ZDAdata {
                talker,
                time,
                day,
                month,
                year,
                local_tz_hour,
                local_tz_minute,
            },
        )),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::VerboseErrorKind::Context;

    type VE<'a> = VerboseError<&'a str>;

    #[test]
    fn test_parse() {
        let parsed = parse::<VE>("$EIGAQ,RMC*2B\r\n").unwrap().1;
        let data = gaq::<VE>("EIGAQ,RMC").unwrap().1;

        assert_eq!(NMEA::GAQ(data), parsed);
    }

    #[test]
    fn test_error_checksum() {
        let input = "$EIGAQ,RMC*2C\r\n";
        let result = parse::<VE>(input);

        if let Err(Err::Failure(mut f)) = result {
            assert_eq!(Context("checksum verification"), f.errors.pop().unwrap().1);
        } else {
            assert!(false, "Did not experience failure")
        }
    }

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
    fn test_message() {
        let parsed = message::<VE>("EIGAQ,RMC").unwrap().1;
        let data = gaq::<VE>("EIGAQ,RMC").unwrap().1;

        assert_eq!(NMEA::GAQ(data), parsed);

        let parsed = message::<VE>("EIGNQ,RMC").unwrap().1;
        let data = gnq::<VE>("EIGNQ,RMC").unwrap().1;

        assert_eq!(NMEA::GNQ(data), parsed);
    }

    #[test]
    fn test_nav_mode() {
        assert_eq!(NavigationMode::FixNone, nav_mode::<VE>("1").unwrap().1);
        assert_eq!(NavigationMode::Fix2D, nav_mode::<VE>("2").unwrap().1);
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

    #[test]
    fn test_glq() {
        let parsed = glq::<VE>("EIGLQ,RMC").unwrap().1;

        assert_eq!(Talker::ECDIS, parsed.talker);
        assert_eq!("RMC".to_string(), parsed.message_id);
    }

    #[test]
    fn test_gnq() {
        let parsed = gnq::<VE>("EIGNQ,RMC").unwrap().1;

        assert_eq!(Talker::ECDIS, parsed.talker);
        assert_eq!("RMC".to_string(), parsed.message_id);
    }

    #[test]
    fn test_gpq() {
        let parsed = gpq::<VE>("EIGPQ,RMC").unwrap().1;

        assert_eq!(Talker::ECDIS, parsed.talker);
        assert_eq!("RMC".to_string(), parsed.message_id);
    }

    #[test]
    fn test_grs() {
        let parsed = grs::<VE>("GNGRS,104148.00,1,2.6,2.2,-1.6,-1.1,-1.7,-1.5,5.8,1.7,,,,,1,1")
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
        assert_eq!(Signal::GPSL1CA, parsed.system);
        assert_eq!(Signal::GPSL1CA, parsed.signal);

        let parsed = grs::<VE>("GNGRS,104148.00,1,,0.0,2.5,0.0,,2.8,,,,,,,1,5")
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
        assert_eq!(Signal::GPSL1CA, parsed.system);
        assert_eq!(Signal::Unknown, parsed.signal);
    }

    #[test]
    fn test_gsa() {
        let parsed = gsa::<VE>("GPGSA,A,3,23,29,07,08,09,18,26,28,,,,,1.94,1.18,1.54,1")
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
        assert_approx_eq!(1.94, parsed.pdop);
        assert_approx_eq!(1.18, parsed.hdop);
        assert_approx_eq!(1.54, parsed.vdop);
        assert_eq!(Signal::GPSL1CA, parsed.system);
    }

    #[test]
    fn test_gst() {
        let parsed = gst::<VE>("GPGST,082356.00,1.8,,,,1.7,1.3,2.2").unwrap().1;

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
        let (_, parsed) = gsv::<VE>("GPGSV,3,1,09,09,,,17,10,,,40,12,,,49,13,,,35,1").unwrap();

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
        assert_eq!(Signal::GPSL1CA, parsed.signal);

        let parsed = gsv::<VE>("GPGSV,3,3,09,25,,,40,1").unwrap().1;

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
        assert_eq!(Signal::GPSL1CA, parsed.signal);

        let parsed = gsv::<VE>("GPGSV,1,1,03,12,,,42,24,,,47,32,,,37,5")
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
        assert_eq!(Signal::Unknown, parsed.signal);

        let parsed = gsv::<VE>("GAGSV,1,1,00,2").unwrap().1;

        let satellites: Vec<GSVsatellite> = vec![];

        assert_eq!(Talker::Galileo, parsed.talker);
        assert_eq!(1, parsed.num_msgs);
        assert_eq!(1, parsed.msg);
        assert_eq!(0, parsed.num_satellites);
        assert_eq!(satellites, parsed.satellites);
        assert_eq!(Signal::GLONASSL1OF, parsed.signal);
    }

    #[test]
    fn test_rmc() {
        let parsed =
            rmc::<VE>("GPRMC,083559.00,A,4717.11437,N,00833.91522,E,0.004,77.52,091202,,,A,V")
                .unwrap()
                .1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_eq!(NaiveTime::from_hms_milli(08, 35, 59, 0), parsed.time);
        assert_eq!(Status::Valid, parsed.status);
        assert_approx_eq!(47.28524, parsed.lat_lon.latitude);
        assert_approx_eq!(8.565253, parsed.lat_lon.longitude);
        assert_approx_eq!(0.004, parsed.speed);
        assert_approx_eq!(77.52, parsed.course_over_ground);
        assert_eq!(NaiveDate::from_ymd(02, 12, 9), parsed.date);
        assert_eq!(None, parsed.magnetic_variation);
        assert_eq!(None, parsed.magnetic_variation_east_west);
        assert_eq!(PositionMode::AutonomousGNSSFix, parsed.position_mode);
        assert_eq!(Status::Invalid, parsed.nav_status);
    }

    #[test]
    fn test_txt() {
        let parsed = txt::<VE>("GPTXT,01,01,02,u-blox ag - www.u-blox.com")
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
        let parsed = vlw::<VE>("GPVLW,,N,,N,15.8,N,1.2,N").unwrap().1;

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
        let parsed = vtg::<VE>("GPVTG,77.52,T,,M,0.004,N,0.008,K,A").unwrap().1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_approx_eq!(77.52, parsed.course_over_ground_true);
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
    fn test_zda() {
        let parsed = zda::<VE>("GPZDA,082710.00,16,09,2002,00,00").unwrap().1;

        assert_eq!(NaiveTime::from_hms_milli(8, 27, 10, 0), parsed.time);
        assert_eq!(16, parsed.day);
        assert_eq!(9, parsed.month);
        assert_eq!(2002, parsed.year);
        assert_eq!(0, parsed.local_tz_hour);
        assert_eq!(0, parsed.local_tz_minute);
    }
}
