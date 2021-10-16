use chrono::naive::NaiveDate;
use chrono::naive::NaiveTime;

use crate::gps::Driver;
use crate::gps::MKTData;
use crate::gps::UBXData;
use crate::nmea::parser_util::*;
use crate::nmea::sentence_parser::parse_sentence;
use crate::nmea::sentence_parser::NMEASentence;
use crate::nmea::EastWest;
use crate::nmea::NorthSouth;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::sequence::*;
use nom::Err;
use nom::IResult;

use std::num::ParseFloatError;
use std::num::ParseIntError;
use std::time::Duration;

type VE<'a> = VerboseError<&'a [u8]>;

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Parser {
    pub driver: Driver,
}

impl Parser {
    pub fn new(driver: Driver) -> Self {
        Parser { driver }
    }

    pub fn parse<'a>(&'a self, input: &'a [u8], received: Duration) -> IResult<&'a [u8], NMEA, VE> {
        parse::<VE>(input, &self.driver, received)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NMEA {
    DTM(DTMData),
    GAQ(GAQData),
    GBQ(GBQData),
    GBS(GBSData),
    GGA(GGAData),
    GLL(GLLData),
    GLQ(GLQData),
    GNQ(GNQData),
    GNS(GNSData),
    GPQ(GPQData),
    GRS(GRSData),
    GSA(GSAData),
    GST(GSTData),
    GSV(GSVData),
    PMKT(MKTData),
    PUBX(UBXData),
    RMC(RMCData),
    TXT(TXTData),
    VLW(VLWData),
    VTG(VTGData),
    ZDA(ZDAData),
    InvalidChecksum(ChecksumMismatch),
    ParseError(String),
    ParseFailure(String),
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumMismatch {
    pub message: String,
    pub given: u8,
    pub calculated: u8,
}

pub(crate) fn parse<
    'a,
    E: ParseError<&'a [u8]>
        + ContextError<&'a [u8]>
        + FromExternalError<&'a [u8], ParseFloatError>
        + FromExternalError<&'a [u8], ParseIntError>,
>(
    input: &'a [u8],
    driver: &Driver,
    received: Duration,
) -> IResult<&'a [u8], NMEA, E> {
    let result = parse_sentence::<VerboseError<&'a [u8]>>(input, received);

    let (input, data) = match result {
        Ok((input, sentence)) => match sentence {
            NMEASentence::InvalidChecksum(cm) => {
                return Ok((input, NMEA::InvalidChecksum(cm)));
            }
            NMEASentence::ParseError(e) => return Ok((input, NMEA::ParseError(e))),
            NMEASentence::Valid(d) => (input, d),
        },
        Err(Err::Incomplete(n)) => {
            return Err(Err::Incomplete(n));
        }
        Err(_) => unreachable!(),
    };

    match message::<VerboseError<&'a str>>(data, driver, received) {
        Err(Err::Error(_)) => Ok((input, NMEA::ParseError(String::from(data)))),
        Err(Err::Failure(_)) => Ok((input, NMEA::ParseFailure(String::from(data)))),
        Err(Err::Incomplete(_)) => unreachable!(
            "Got Incomplete when complete parsers were used on: {:?}",
            data
        ),
        // discard input from sub-parser, it was fully consumed
        Ok((_, nmea)) => Ok((input, nmea)),
    }
}

pub fn message<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
    driver: &Driver,
    received: Duration,
) -> IResult<&'a str, NMEA, E> {
    match nmea_message::<E>(input, received) {
        Ok(r) => Ok(r),
        Err(_) => private_message(input, driver),
    }
}

pub fn nmea_message<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
    received: Duration,
) -> IResult<&'a str, NMEA, E> {
    alt((
        map(dtm, |mut msg: DTMData| {
            msg.received = Some(received);
            NMEA::DTM(msg)
        }),
        map(gaq, |mut msg: GAQData| {
            msg.received = Some(received);
            NMEA::GAQ(msg)
        }),
        map(gbq, |mut msg: GBQData| {
            msg.received = Some(received);
            NMEA::GBQ(msg)
        }),
        map(gbs, |mut msg: GBSData| {
            msg.received = Some(received);
            NMEA::GBS(msg)
        }),
        map(gga, |mut msg: GGAData| {
            msg.received = Some(received);
            NMEA::GGA(msg)
        }),
        map(gll, |mut msg: GLLData| {
            msg.received = Some(received);
            NMEA::GLL(msg)
        }),
        map(glq, |mut msg: GLQData| {
            msg.received = Some(received);
            NMEA::GLQ(msg)
        }),
        map(gnq, |mut msg: GNQData| {
            msg.received = Some(received);
            NMEA::GNQ(msg)
        }),
        map(gns, |mut msg: GNSData| {
            msg.received = Some(received);
            NMEA::GNS(msg)
        }),
        map(gpq, |mut msg: GPQData| {
            msg.received = Some(received);
            NMEA::GPQ(msg)
        }),
        map(grs, |mut msg: GRSData| {
            msg.received = Some(received);
            NMEA::GRS(msg)
        }),
        map(gsa, |mut msg: GSAData| {
            msg.received = Some(received);
            NMEA::GSA(msg)
        }),
        map(gst, |mut msg: GSTData| {
            msg.received = Some(received);
            NMEA::GST(msg)
        }),
        map(gsv, |mut msg: GSVData| {
            msg.received = Some(received);
            NMEA::GSV(msg)
        }),
        map(rmc, |mut msg: RMCData| {
            msg.received = Some(received);
            NMEA::RMC(msg)
        }),
        map(txt, |mut msg: TXTData| {
            msg.received = Some(received);
            NMEA::TXT(msg)
        }),
        map(vlw, |mut msg: VLWData| {
            msg.received = Some(received);
            NMEA::VLW(msg)
        }),
        map(vtg, |mut msg: VTGData| {
            msg.received = Some(received);
            NMEA::VTG(msg)
        }),
        map(zda, |mut msg: ZDAData| {
            msg.received = Some(received);
            NMEA::ZDA(msg)
        }),
    ))(input)
}

pub(crate) fn private_message<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
    driver: &Driver,
) -> IResult<&'a str, NMEA, E> {
    driver.parse_private(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    Error,
    Notice,
    User,
    Warning,
    Unknown(u32),
}

pub(crate) fn msg_type<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, MessageType, E> {
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

pub(crate) fn nav_mode<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, NavigationMode, E> {
    map(one_of("123"), |c| match c {
        '1' => NavigationMode::FixNone,
        '2' => NavigationMode::Fix2D,
        '3' => NavigationMode::Fix3D,
        _ => panic!("Unhandled navigation mode {:?}", c),
    })(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum OperationMode {
    Automatic,
    Manual,
}

pub(crate) fn op_mode<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, OperationMode, E> {
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

pub(crate) fn pos_mode<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, PositionMode, E> {
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
    Fix2D,
    Fix3D,
}

impl Default for Quality {
    fn default() -> Self {
        Quality::NoFix
    }
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

    Unknown,
}

pub(crate) fn signal<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, Signal, E> {
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

pub(crate) fn system<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, System, E> {
    map(uint32, |c| match c {
        1 => System::GPS,
        2 => System::GLONASS,
        3 => System::Galileo,
        4 => System::BeiDuo,
        5 => System::QZSS,
        _ => System::Unknown,
    })(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Talker {
    BeiDuo,
    Combination,
    ECDIS,
    GLONASS,
    GPS,
    Galileo,
    Private,
    Unknown(String),
}

pub(crate) fn talker<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Talker, E> {
    map(
        alt((tag("P"), take_while_m_n(2, 2, is_upper_alphanum))),
        |t| match t {
            "EI" => Talker::ECDIS,
            "GA" => Talker::Galileo,
            "GB" => Talker::BeiDuo,
            "GL" => Talker::GLONASS,
            "GN" => Talker::Combination,
            "GP" => Talker::GPS,
            "P" => Talker::Private,
            _ => Talker::Unknown(t.to_string()),
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct DTMData {
    pub received: Option<Duration>,
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

pub(crate) fn dtm<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseFloatError>,
>(
    input: &'a str,
) -> IResult<&'a str, DTMData, E> {
    parse_message(
        "DTM",
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
        |(talker, datum, sub_datum, lat, north_south, lon, east_west, alt, ref_datum)| DTMData {
            received: None,
            talker,
            datum,
            sub_datum,
            lat,
            north_south,
            lon,
            east_west,
            alt,
            ref_datum,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GAQData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gaq<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, GAQData, E> {
    context(
        "GAQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GAQ"), preceded(comma, any)))),
            |(talker, message_id)| GAQData {
                received: None,
                talker,
                message_id,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBQData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gbq<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, GBQData, E> {
    parse_message(
        "GBQ",
        tuple((talker, preceded(tag("GBQ"), preceded(comma, any)))),
        |(talker, message_id)| GBQData {
            received: None,
            talker,
            message_id,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBSData {
    pub received: Option<Duration>,
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

pub(crate) fn gbs<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GBSData, E> {
    parse_message(
        "GBS",
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
        |(talker, time, err_lat, err_lon, err_alt, svid, prob, bias, stddev, system, signal)| {
            GBSData {
                received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GGAData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub time: NaiveTime,
    pub lat_lon: Option<LatLon>,
    pub quality: Quality,
    pub num_satellites: u32,
    pub hdop: Option<f32>,
    pub alt: Option<f32>,
    pub alt_unit: String,
    pub sep: Option<f32>,
    pub sep_unit: String,
    pub diff_age: Option<u32>,
    pub diff_station: Option<u32>,
}

pub(crate) fn gga<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GGAData, E> {
    parse_message(
        "GGA",
        tuple((
            terminated(talker, terminated(tag("GGA"), comma)),
            terminated(time, comma),
            terminated(latlon, comma),
            terminated(quality, comma),
            terminated(uint32, comma),
            terminated(opt(flt32), comma),
            terminated(opt(flt32), comma),
            terminated(any, comma),
            terminated(opt(flt32), comma),
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
        )| GGAData {
            received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GLLData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub lat_lon: Option<LatLon>,
    pub time: NaiveTime,
    pub status: Status,
    pub position_mode: PositionMode,
}

pub(crate) fn gll<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GLLData, E> {
    parse_message(
        "GLL",
        tuple((
            terminated(talker, tag("GLL")),
            preceded(comma, latlon),
            preceded(comma, time),
            preceded(comma, status),
            preceded(comma, pos_mode),
        )),
        |(talker, lat_lon, time, status, position_mode)| GLLData {
            received: None,
            talker,
            lat_lon,
            time,
            status,
            position_mode,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GLQData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn glq<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, GLQData, E> {
    parse_message(
        "GLQ",
        tuple((talker, preceded(tag("GLQ"), preceded(comma, any)))),
        |(talker, message_id)| GLQData {
            received: None,
            talker,
            message_id,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GNQData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gnq<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, GNQData, E> {
    parse_message(
        "GNQ",
        tuple((talker, preceded(tag("GNQ"), preceded(comma, any)))),
        |(talker, message_id)| GNQData {
            received: None,
            talker,
            message_id,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GNSData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub time: NaiveTime,
    pub lat_lon: Option<LatLon>,
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

pub(crate) fn gns<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GNSData, E> {
    parse_message(
        "GNS",
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
        )| GNSData {
            received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GPQData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gpq<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, GPQData, E> {
    parse_message(
        "GPQ",
        tuple((talker, preceded(tag("GPQ"), preceded(comma, any)))),
        |(talker, message_id)| GPQData {
            received: None,
            talker,
            message_id,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GRSData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub time: NaiveTime,
    pub gga_includes_residuals: bool,
    pub residuals: Vec<Option<f32>>,
    pub system: System,
    pub signal: Option<Signal>,
}

pub(crate) fn grs<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GRSData, E> {
    parse_message(
        "GRS",
        tuple((
            terminated(talker, tag("GRS")),
            preceded(comma, time),
            preceded(comma, map(one_of("10"), |c| c == '1')),
            map(many_m_n(12, 12, preceded(comma, opt(flt32))), Vec::from),
            preceded(comma, system),
            preceded(comma, opt(signal)),
        )),
        |(talker, time, gga_includes_residuals, residuals, system, signal)| GRSData {
            received: None,
            talker,
            time,
            gga_includes_residuals,
            residuals,
            system,
            signal,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GSAData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub operation_mode: OperationMode,
    pub navigation_mode: NavigationMode,
    pub satellite_ids: Vec<Option<u32>>,
    pub pdop: Option<f32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub system: Option<System>,
}

pub(crate) fn gsa<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GSAData, E> {
    parse_message(
        "GSA",
        tuple((
            terminated(talker, terminated(tag("GSA"), comma)),
            terminated(op_mode, comma),
            terminated(nav_mode, comma),
            map(many_m_n(12, 12, terminated(opt(uint32), comma)), Vec::from),
            terminated(opt(flt32), comma),
            terminated(opt(flt32), comma),
            opt(flt32),
            opt(preceded(comma, system)),
        )),
        |(talker, operation_mode, navigation_mode, satellite_ids, pdop, hdop, vdop, system)| {
            GSAData {
                received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GSTData {
    pub received: Option<Duration>,
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

pub(crate) fn gst<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GSTData, E> {
    parse_message(
        "GST",
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
        )| GSTData {
            received: None,
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
    )(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GSVsatellite {
    pub id: u32,
    pub elevation: Option<u32>,
    pub azimuth: Option<u32>,
    pub cno: Option<u32>,
}

pub(crate) fn gsv_sat<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GSVsatellite, E> {
    context(
        "GSV satellite",
        map(
            tuple((
                preceded(comma, uint32),
                preceded(comma, opt(uint32)),
                preceded(comma, opt(uint32)),
                preceded(comma, opt(uint32)),
            )),
            |(id, elevation, azimuth, cno)| GSVsatellite {
                id,
                elevation,
                azimuth,
                cno,
            },
        ),
    )(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GSVData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub num_msgs: u32,
    pub msg: u32,
    pub num_satellites: u32,
    pub satellites: Vec<GSVsatellite>,
    pub signal: Option<Signal>,
}

pub(crate) fn gsv<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, GSVData, E> {
    parse_message(
        "GSV",
        tuple((
            terminated(talker, tag("GSV")),
            preceded(comma, uint32),
            preceded(comma, uint32),
            preceded(comma, uint32),
            many_m_n(0, 4, gsv_sat),
            opt(preceded(comma, opt(signal))),
        )),
        |(talker, num_msgs, msg, num_satellites, satellites, signal)| GSVData {
            received: None,
            talker,
            num_msgs,
            msg,
            num_satellites,
            satellites,
            signal: signal.unwrap_or(None),
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RMCData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub time: NaiveTime,
    pub status: Status,
    pub lat_lon: Option<LatLon>,
    pub speed: f32,
    pub course_over_ground: Option<f32>,
    pub date: NaiveDate,
    pub magnetic_variation: Option<f32>,
    pub magnetic_variation_east_west: Option<EastWest>,
    pub position_mode: PositionMode,
    pub nav_status: Option<Status>,
}

pub(crate) fn rmc<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ParseFloatError>,
>(
    input: &'a str,
) -> IResult<&'a str, RMCData, E> {
    parse_message(
        "RMC",
        tuple((
            terminated(talker, tag("RMC")),
            preceded(comma, time),
            preceded(comma, status),
            preceded(comma, latlon),
            preceded(comma, flt32),
            preceded(comma, opt(flt32)),
            preceded(comma, date),
            preceded(comma, opt(flt32)),
            preceded(comma, opt(east_west)),
            preceded(comma, pos_mode),
            opt(preceded(comma, status)),
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
        )| RMCData {
            received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TXTData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub num_msgs: u32,
    pub msg: u32,
    pub msg_type: MessageType,
    pub text: String,
}

pub(crate) fn txt<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, TXTData, E> {
    parse_message(
        "TXT",
        tuple((
            terminated(talker, terminated(tag("TXT"), comma)),
            terminated(uint32, comma),
            terminated(uint32, comma),
            terminated(msg_type, comma),
            any,
        )),
        |(talker, num_msgs, msg, msg_type, text)| TXTData {
            received: None,
            talker,
            num_msgs,
            msg,
            msg_type,
            text,
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VLWData {
    pub received: Option<Duration>,
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

pub(crate) fn vlw<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseFloatError>,
>(
    input: &'a str,
) -> IResult<&'a str, VLWData, E> {
    parse_message(
        "VLW",
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
        )| VLWData {
            received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VTGData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub course_over_ground_true: Option<f32>,
    pub course_over_ground_true_unit: String,
    pub course_over_ground_magnetic: Option<f32>,
    pub course_over_ground_magnetic_unit: String,
    pub speed_over_ground_knots: f32,
    pub speed_over_ground_knots_unit: String,
    pub speed_over_ground_km: f32,
    pub speed_over_ground_km_unit: String,
    pub position_mode: PositionMode,
}

pub(crate) fn vtg<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseFloatError>,
>(
    input: &'a str,
) -> IResult<&'a str, VTGData, E> {
    parse_message(
        "VTG",
        tuple((
            terminated(talker, tag("VTG")),
            preceded(comma, opt(flt32)),
            preceded(comma, any),
            preceded(comma, opt(flt32)),
            preceded(comma, any),
            preceded(comma, flt32),
            preceded(comma, any),
            preceded(comma, flt32),
            preceded(comma, any),
            preceded(comma, pos_mode),
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
        )| VTGData {
            received: None,
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
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZDAData {
    pub received: Option<Duration>,
    pub talker: Talker,
    pub time: Option<NaiveTime>,
    pub day: Option<u32>,
    pub month: Option<u32>,
    pub year: Option<i32>,
    pub local_tz_hour: i32,
    pub local_tz_minute: u32,
}

pub(crate) fn zda<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, ZDAData, E> {
    parse_message(
        "ZDA",
        tuple((
            terminated(talker, terminated(tag("ZDA"), comma)),
            terminated(opt(time), comma),
            terminated(opt(uint32), comma),
            terminated(opt(uint32), comma),
            terminated(opt(int32), comma),
            terminated(int32, comma),
            uint32,
        )),
        |(talker, time, day, month, year, local_tz_hour, local_tz_minute)| ZDAData {
            received: None,
            talker,
            time,
            day,
            month,
            year,
            local_tz_hour,
            local_tz_minute,
        },
    )(input)
}
