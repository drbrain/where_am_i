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
    Unsupported(String),
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

pub(crate) fn message<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NMEA, E> {
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
        map(rest, |m: &str| NMEA::Unsupported(m.to_string())),
    ))(input)
}

pub(crate) fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    map(take_while(|c| c != ','), |m: &str| m.to_string())(input)
}

pub(crate) fn checksum_check(data: &str, expected: &str) -> bool {
    let expected = u8::from_str_radix(expected, 16).unwrap();

    expected == data.bytes().fold(0, |cs, b| cs ^ b)
}

pub(crate) fn checksum_verify<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
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

pub(crate) fn comma<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(",")(input)
}

pub(crate) fn date<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NaiveDate, E> {
    map(
        tuple((two_digit, two_digit, two_digit_i)),
        |(day, month, year)| NaiveDate::from_ymd(year, month, day),
    )(input)
}

pub(crate) fn dollar<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("$")(input)
}

pub(crate) fn dot<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(".")(input)
}

pub(crate) fn eol<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("\r\n")(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum EastWest {
    East,
    West,
}

pub(crate) fn east_west<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, EastWest, E> {
    map(one_of("EW"), |ew| match ew {
        'E' => EastWest::East,
        'W' => EastWest::West,
        _ => panic!("Unknown direction {:?}", ew),
    })(input)
}

pub(crate) fn flt32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map_res(recognize_float, |s: &str| s.parse())(input)
}

pub(crate) fn int32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i32, E> {
    map_res(
        recognize(preceded(opt(char('-')), take_while(is_digit))),
        |s: &str| s.parse(),
    )(input)
}

pub(crate) fn is_digit(chr: char) -> bool {
    chr.is_ascii_digit()
}

pub(crate) fn is_upper_alphanum(chr: char) -> bool {
    chr.is_ascii_uppercase() || chr.is_ascii_digit()
}

#[derive(Clone, Debug, PartialEq)]
pub enum NorthSouth {
    North,
    South,
}

pub(crate) fn lat<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map(tuple((two_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

pub(crate) fn lon<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map(tuple((three_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatLon {
    pub latitude: f32,
    pub longitude: f32,
}

pub(crate) fn latlon<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, LatLon, E> {
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

pub(crate) fn msg_type<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, MessageType, E> {
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

pub(crate) fn nav_mode<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NavigationMode, E> {
    map(one_of("123"), |c| match c {
        '1' => NavigationMode::FixNone,
        '2' => NavigationMode::Fix2D,
        '3' => NavigationMode::Fix3D,
        _ => panic!("Unhandled navigation mode {:?}", c),
    })(input)
}

pub(crate) fn north_south<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NorthSouth, E> {
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

pub(crate) fn op_mode<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, OperationMode, E> {
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

pub(crate) fn pos_mode<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, PositionMode, E> {
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

pub(crate) fn quality<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Quality, E> {
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
    // GPS L1C/A
    // SBAS L1C/A
    // BeiDou B1I D1
    // BeiDou B1I D1
    // QZSS L1C/A
    // GLONASS L1 OF
    L1,

    // Galileo E5 bI
    // Galileo E5 bQ
    E5,

    // GLONASS L2 OF
    L2OF,

    // QZSS L1S
    L1S,

    // GPS L2 CM
    // QZSS L2 CM
    L2CM,

    // GPS L2 CL
    // QZSS L2 CL
    L2CL,

    // Galileo E1 C
    // Galileo E1 B
    E1,

    // BeiDou B2I D1
    // BeiDou B2I D1
    B2I,

    Unknown
}

pub(crate) fn signal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Signal, E> {
    map(uint32, |c| match c {
        1 => Signal::L1,
        2 => Signal::E5,
        3 => Signal::L2OF,
        4 => Signal::L1S,
        5 => Signal::L2CM,
        6 => Signal::L2CL,
        7 => Signal::E1,
        11 => Signal::B2I,
        _ => Signal::Unknown,
    })(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Valid,
    Invalid,
}

pub(crate) fn status<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Status, E> {
    map(one_of("AV"), |c| match c {
        'A' => Status::Valid,
        'V' => Status::Invalid,
        _ => panic!("Unhandled quality {:?}", c),
    })(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum System {
    BeiDuo,
    GLONASS,
    GPS,
    Galileo,
    QZSS,
    Unknown,
}

pub(crate) fn system<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, System, E> {
    map(uint32, |c| match c {
        1 => System::GPS,
        2 => System::GLONASS,
        3 => System::Galileo,
        4 => System::BeiDuo,
        5 => System::QZSS,
        _ => System::Unknown,
    })(input)
}

pub(crate) fn star<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
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

pub(crate) fn talker<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Talker, E> {
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

pub(crate) fn three_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(3, 3, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn time<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NaiveTime, E> {
    map(
        tuple((two_digit, two_digit, two_digit, preceded(dot, two_digit))),
        |(hour, minute, second, subsec)| {
            NaiveTime::from_hms_milli(hour, minute, second, subsec * 100)
        },
    )(input)
}

pub(crate) fn two_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn two_digit_i<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i32, E> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn uint32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
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

pub(crate) fn dtm<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, DTMdata, E> {
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

pub(crate) fn gaq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GAQdata, E> {
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

pub(crate) fn gbq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBQdata, E> {
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
    pub system: Option<System>,
    pub signal: Option<Signal>,
}

pub(crate) fn gbs<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBSdata, E> {
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

pub(crate) fn gga<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GGAdata, E> {
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

pub(crate) fn gll<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GLLdata, E> {
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

pub(crate) fn glq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GLQdata, E> {
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

pub(crate) fn gnq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GNQdata, E> {
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

pub(crate) fn gns<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GNSdata, E> {
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

pub(crate) fn gpq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GPQdata, E> {
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
    pub system: System,
    pub signal: Signal,
}

pub(crate) fn grs<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GRSdata, E> {
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
    pub system: System,
}

pub(crate) fn gsa<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSAdata, E> {
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

pub(crate) fn gst<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSTdata, E> {
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

pub(crate) fn gsv_sat<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSVsatellite, E> {
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

pub(crate) fn gsv<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSVdata, E> {
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

pub(crate) fn rmc<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, RMCdata, E> {
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

pub(crate) fn txt<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, TXTdata, E> {
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

pub(crate) fn vlw<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VLWdata, E> {
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

pub(crate) fn vtg<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VTGdata, E> {
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

pub(crate) fn zda<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, ZDAdata, E> {
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

