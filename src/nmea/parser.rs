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

use serde::Serialize;

use tracing::trace;

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

pub fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], NMEA, E> {
    use nom::bytes::streaming::tag;

    let result = delimited(
        preceded(garbage, tag(b"$")),
        tuple((terminated(non_star, star), checksum)),
        terminated(opt(tag(b"\r")), tag(b"\n")),
    )(input);

    let (input, (data, given)) = match result {
        Err(Err::Incomplete(_)) => {
            return Err(result.err().unwrap());
        }
        Err(Err::Error(_)) => panic!("Some error parsing: {:?}", input),
        Err(Err::Failure(_)) => panic!("Some failure parsing: {:?}", input),
        Ok(t) => t,
    };

    let calculated = data.iter().fold(0, |c, b| c ^ b);
    let data = std::str::from_utf8(data).unwrap();

    if given == calculated {
        trace!("parsing \"{}\" (checksum OK)", data);

        match message::<VerboseError<&'a str>>(data) {
            Err(Err::Error(_)) => Ok((input, NMEA::ParseError(String::from(data)))),
            Err(Err::Failure(_)) => Ok((input, NMEA::ParseFailure(String::from(data)))),
            Err(Err::Incomplete(_)) => panic!(
                "Got Incomplete when complete parsers were used on: {:?}",
                data
            ),
            // discard input from sub-parser, it was fully consumed
            Ok((_, nmea)) => Ok((input, nmea)),
        }
    } else {
        trace!(
            "invalid checksum for \"{}\" ({} != {})",
            data,
            given,
            calculated
        );

        let message = String::from(data);

        Ok((
            input,
            NMEA::InvalidChecksum(ChecksumMismatch {
                message,
                given,
                calculated,
            }),
        ))
    }
}

pub(crate) fn message<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NMEA, E> {
    alt((
        map(dtm, NMEA::DTM),
        map(gaq, NMEA::GAQ),
        map(gbq, NMEA::GBQ),
        map(gbs, NMEA::GBS),
        map(gga, NMEA::GGA),
        map(gll, NMEA::GLL),
        map(glq, NMEA::GLQ),
        map(gnq, NMEA::GNQ),
        map(gns, NMEA::GNS),
        map(gpq, NMEA::GPQ),
        map(grs, NMEA::GRS),
        map(gsa, NMEA::GSA),
        map(gst, NMEA::GST),
        map(gsv, NMEA::GSV),
        map(rmc, NMEA::RMC),
        map(txt, NMEA::TXT),
        map(vlw, NMEA::VLW),
        map(vtg, NMEA::VTG),
        map(zda, NMEA::ZDA),
        private_message,
    ))(input)
}

pub(crate) fn private_message<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, NMEA, E> {
    alt((
        map(pmkt, NMEA::PMKT),
        map(pubx, NMEA::PUBX),
        map(rest, |m: &str| NMEA::Unsupported(m.to_string())),
    ))(input)
}

pub(crate) fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    map(take_while(|c| c != ','), |m: &str| m.to_string())(input)
}

pub(crate) fn star<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E> {
    use nom::bytes::streaming::tag;

    tag(b"*")(input)
}

pub(crate) fn non_star<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], &'a [u8], E> {
    use nom::bytes::streaming::take_till;

    recognize(take_till(|c| c == b'*'))(input)
}

pub(crate) fn checksum<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], u8, E> {
    use nom::bytes::streaming::take_while_m_n;
    use nom::character::is_hex_digit;

    map(recognize(take_while_m_n(2, 2, is_hex_digit)), |c| {
        u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap()
    })(input)
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

pub(crate) fn dot<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(".")(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum EastWest {
    East,
    West,
}

pub(crate) fn east_west<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, EastWest, E> {
    map(one_of("EW"), |ew| match ew {
        'E' => EastWest::East,
        'W' => EastWest::West,
        _ => panic!("Unknown direction {:?}", ew),
    })(input)
}

pub(crate) fn flt32<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    map_res(recognize_float, |s: &str| s.parse())(input)
}

pub(crate) fn garbage<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], usize, E> {
    use nom::bytes::streaming::tag;
    use nom::bytes::streaming::take_while_m_n;

    context(
        "garbage",
        cut(terminated(
            map(take_while_m_n(0, 164, |c| c != b'$'), |g: &[u8]| g.len()),
            peek(tag(b"$")),
        )),
    )(input)
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

pub(crate) fn latlon<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, Option<LatLon>, E> {
    map(
        tuple((
            map(
                tuple((
                    terminated(opt(lat), comma),
                    terminated(opt(north_south), comma),
                )),
                |(l, d)| match (l, d) {
                    (Some(l), Some(d)) => Some(l * if d == NorthSouth::North { 1.0 } else { -1.0 }),
                    _ => None,
                },
            ),
            map(
                tuple((terminated(opt(lon), comma), opt(east_west))),
                |(l, d)| match (l, d) {
                    (Some(l), Some(d)) => Some(l * if d == EastWest::East { 1.0 } else { -1.0 }),
                    _ => None,
                },
            ),
        )),
        |(latitude, longitude)| match (latitude, longitude) {
            (Some(latitude), Some(longitude)) => Some(LatLon {
                latitude,
                longitude,
            }),
            _ => None,
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

pub(crate) fn msg_type<'a, E: ParseError<&'a str>>(
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

pub(crate) fn north_south<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, NorthSouth, E> {
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

pub(crate) fn three_digit<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(3, 3, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn time<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NaiveTime, E> {
    map(
        tuple((two_digit, two_digit, two_digit, preceded(dot, uint32))),
        |(hour, minute, second, subsec)| {
            NaiveTime::from_hms_milli(hour, minute, second, subsec * 10)
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
pub struct DTMData {
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

pub(crate) fn dtm<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, DTMData, E> {
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
                DTMData {
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
pub struct GAQData {
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gaq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GAQData, E> {
    context(
        "GAQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GAQ"), preceded(comma, any)))),
            |(talker, message_id)| GAQData { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBQData {
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gbq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBQData, E> {
    context(
        "GBQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GBQ"), preceded(comma, any)))),
            |(talker, message_id)| GBQData { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GBSData {
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

pub(crate) fn gbs<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GBSData, E> {
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
                GBSData {
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
pub struct GGAData {
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

pub(crate) fn gga<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GGAData, E> {
    context(
        "GGA",
        all_consuming(map(
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
pub struct GLLData {
    pub talker: Talker,
    pub lat_lon: Option<LatLon>,
    pub time: NaiveTime,
    pub status: Status,
    pub position_mode: PositionMode,
}

pub(crate) fn gll<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GLLData, E> {
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
            |(talker, lat_lon, time, status, position_mode)| GLLData {
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
pub struct GLQData {
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn glq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GLQData, E> {
    context(
        "GLQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GLQ"), preceded(comma, any)))),
            |(talker, message_id)| GLQData { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GNQData {
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gnq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GNQData, E> {
    context(
        "GNQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GNQ"), preceded(comma, any)))),
            |(talker, message_id)| GNQData { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GNSData {
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

pub(crate) fn gns<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GNSData, E> {
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
            )| GNSData {
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
pub struct GPQData {
    pub talker: Talker,
    pub message_id: String,
}

pub(crate) fn gpq<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GPQData, E> {
    context(
        "GPQ",
        all_consuming(map(
            tuple((talker, preceded(tag("GPQ"), preceded(comma, any)))),
            |(talker, message_id)| GPQData { talker, message_id },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GRSData {
    pub talker: Talker,
    pub time: NaiveTime,
    pub gga_includes_residuals: bool,
    pub residuals: Vec<Option<f32>>,
    pub system: System,
    pub signal: Option<Signal>,
}

pub(crate) fn grs<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GRSData, E> {
    context(
        "GRS",
        all_consuming(map(
            tuple((
                terminated(talker, tag("GRS")),
                preceded(comma, time),
                preceded(comma, map(one_of("10"), |c| c == '1')),
                map(many_m_n(12, 12, preceded(comma, opt(flt32))), Vec::from),
                preceded(comma, system),
                preceded(comma, opt(signal)),
            )),
            |(talker, time, gga_includes_residuals, residuals, system, signal)| GRSData {
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
pub struct GSAData {
    pub talker: Talker,
    pub operation_mode: OperationMode,
    pub navigation_mode: NavigationMode,
    pub satellite_ids: Vec<Option<u32>>,
    pub pdop: Option<f32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub system: Option<System>,
}

pub(crate) fn gsa<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSAData, E> {
    context(
        "GSA",
        all_consuming(map(
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
pub struct GSTData {
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

pub(crate) fn gst<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSTData, E> {
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
            )| GSTData {
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

pub(crate) fn gsv_sat<'a, E: ParseError<&'a str>>(
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
    pub talker: Talker,
    pub num_msgs: u32,
    pub msg: u32,
    pub num_satellites: u32,
    pub satellites: Vec<GSVsatellite>,
    pub signal: Option<Signal>,
}

pub(crate) fn gsv<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, GSVData, E> {
    context(
        "GSV",
        all_consuming(map(
            tuple((
                terminated(talker, tag("GSV")),
                preceded(comma, uint32),
                preceded(comma, uint32),
                preceded(comma, uint32),
                many_m_n(0, 4, gsv_sat),
                opt(preceded(comma, opt(signal))),
            )),
            |(talker, num_msgs, msg, num_satellites, satellites, signal)| GSVData {
                talker,
                num_msgs,
                msg,
                num_satellites,
                satellites,
                signal: signal.unwrap_or(None),
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MKTData {
    SystemMessage(MKTSystemMessage),
    TextMessage(MKTTextMessage),
}

pub(crate) fn pmkt<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, MKTData, E> {
    context(
        "PMKT",
        alt((
            map(mkt_010, MKTData::SystemMessage),
            map(mkt_011, MKTData::TextMessage),
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MKTSystemMessage {
    Unknown,
    Startup,
    ExtendedPredictionOrbit,
    Normal,
    Unhandled(u32),
}

pub(crate) fn mkt_010<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, MKTSystemMessage, E> {
    context(
        "MKT 010",
        all_consuming(map(
            preceded(preceded(tag("PMTK010"), comma), uint32),
            |m| match m {
                0 => MKTSystemMessage::Unknown,
                1 => MKTSystemMessage::Startup,
                2 => MKTSystemMessage::ExtendedPredictionOrbit,
                3 => MKTSystemMessage::Normal,
                u => MKTSystemMessage::Unhandled(u),
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MKTTextMessage {
    pub message: String,
}

pub(crate) fn mkt_011<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, MKTTextMessage, E> {
    context(
        "MKT 011",
        all_consuming(map(preceded(preceded(tag("PMTK011"), comma), rest), |m| {
            MKTTextMessage {
                message: m.to_string(),
            }
        })),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RMCData {
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

pub(crate) fn rmc<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, RMCData, E> {
    context(
        "RMC",
        all_consuming(map(
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
pub struct TXTData {
    pub talker: Talker,
    pub num_msgs: u32,
    pub msg: u32,
    pub msg_type: MessageType,
    pub text: String,
}

pub(crate) fn txt<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, TXTData, E> {
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
            |(talker, num_msgs, msg, msg_type, text)| TXTData {
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
pub enum UBXData {
    Position(UBXPosition),
    Satellites(UBXSatellites),
    Time(UBXTime),
}

pub(crate) fn pubx<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, UBXData, E> {
    context(
        "PUBX",
        alt((
            map(ubx_00, UBXData::Position),
            map(ubx_03, UBXData::Satellites),
            map(ubx_04, UBXData::Time),
        )),
    )(input)
}

#[derive(Clone, Eq, Debug, PartialEq)]
pub enum UBXPort {
    I2C = 0,
    USART1 = 1,
    USART2 = 2,
    USB = 3,
    SPI = 4,
}

bitflags! {
    pub struct UBXPortMask: u16 {
    const I2C = 0x0000;
    const USART1 = 0x0001;
    const USART2 = 0x0102;
    const USB = 0x0003;
    const SPI = 0x0004;
    }
}

#[derive(Clone, Eq, Debug, PartialEq, Serialize)]
pub struct UBXConfig {
    pub port: UBXPort,
    pub in_proto: UBXPortMask,
    pub out_proto: UBXPortMask,
    pub baudrate: u32,
    pub autobauding: bool,
}

#[derive(Clone, Eq, Debug, PartialEq, Serialize)]
pub struct UBXPositionPoll {}

#[derive(Clone, Eq, Debug, PartialEq)]
pub enum UBXNavigationStatus {
    NoFix,
    DeadRecokning,
    Standalone2D,
    Standalone3D,
    Differential2D,
    Differential3D,
    Combined,
    TimeOnly,
    Unknown(String),
}

pub(crate) fn ubx_nav_stat<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, UBXNavigationStatus, E> {
    context(
        "UBX navigation status",
        map(take_while_m_n(2, 2, is_upper_alphanum), |ns| match ns {
            "NF" => UBXNavigationStatus::NoFix,
            "DR" => UBXNavigationStatus::DeadRecokning,
            "G2" => UBXNavigationStatus::Standalone2D,
            "G3" => UBXNavigationStatus::Standalone3D,
            "D2" => UBXNavigationStatus::Differential2D,
            "D3" => UBXNavigationStatus::Differential3D,
            "RK" => UBXNavigationStatus::Combined,
            "TT" => UBXNavigationStatus::TimeOnly,
            u => UBXNavigationStatus::Unknown(String::from(u)),
        }),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct UBXPosition {
    pub time: NaiveTime,
    pub lat_lon: Option<LatLon>,
    pub alt_ref: f32,
    pub nav_status: UBXNavigationStatus,
    pub horizontal_accuracy: f32,
    pub vertical_accuracy: f32,
    pub speed_over_ground: f32,
    pub course_over_ground: f32,
    pub vertical_velocity: f32,
    pub diff_age: Option<u32>,
    pub hdop: f32,
    pub vdop: f32,
    pub tdop: f32,
    pub num_satellites: u32,
    pub reserved: u32,
    pub dead_reckoning: bool,
}

pub(crate) fn ubx_00<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, UBXPosition, E> {
    context(
        "UBX 00",
        all_consuming(map(
            tuple((
                preceded(
                    tag("PUBX"),
                    preceded(comma, preceded(tag("00"), preceded(comma, time))),
                ),
                preceded(comma, latlon),
                preceded(comma, flt32),
                preceded(comma, ubx_nav_stat),
                preceded(comma, flt32),
                preceded(comma, flt32),
                preceded(comma, flt32),
                preceded(comma, flt32),
                preceded(comma, flt32),
                preceded(comma, opt(uint32)),
                preceded(comma, flt32),
                preceded(comma, flt32),
                preceded(comma, flt32),
                preceded(comma, uint32),
                preceded(comma, uint32),
                preceded(comma, map(uint32, |dr| dr == 1)),
            )),
            |(
                time,
                lat_lon,
                alt_ref,
                nav_status,
                horizontal_accuracy,
                vertical_accuracy,
                speed_over_ground,
                course_over_ground,
                vertical_velocity,
                diff_age,
                hdop,
                vdop,
                tdop,
                num_satellites,
                reserved,
                dead_reckoning,
            )| UBXPosition {
                time,
                lat_lon,
                alt_ref,
                nav_status,
                horizontal_accuracy,
                vertical_accuracy,
                speed_over_ground,
                course_over_ground,
                vertical_velocity,
                diff_age,
                hdop,
                vdop,
                tdop,
                num_satellites,
                reserved,
                dead_reckoning,
            },
        )),
    )(input)
}

#[derive(Clone, Eq, Debug, PartialEq, Serialize)]
pub struct UBXRate {
    pub message: String,
    pub rddc: u32,
    pub rus1: u32,
    pub rus2: u32,
    pub rusb: u32,
    pub rspi: u32,
    pub reserved: u32,
}

#[derive(Clone, Eq, Debug, PartialEq, Serialize)]
pub struct UBXSvsPoll {}

#[derive(Clone, Eq, Debug, PartialEq)]
pub enum UBXSatelliteStatus {
    NotUsed,
    Used,
    EphemerisAvailable,
}

pub(crate) fn ubx_sat_status<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, UBXSatelliteStatus, E> {
    map(one_of("-Ue"), |c| match c {
        '-' => UBXSatelliteStatus::NotUsed,
        'U' => UBXSatelliteStatus::Used,
        'e' => UBXSatelliteStatus::EphemerisAvailable,
        _ => panic!("Unknown UBX satellite status {:?}", c),
    })(input)
}

#[derive(Clone, Eq, Debug, PartialEq)]
pub struct UBXSatellite {
    pub id: u32,
    pub status: UBXSatelliteStatus,
    pub azimuth: Option<u32>,
    pub elevation: Option<u32>,
    pub cno: u32,
    pub lock_time: u32,
}

pub(crate) fn ubx_satellite<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, UBXSatellite, E> {
    context(
        "UBX satellite",
        map(
            tuple((
                preceded(comma, uint32),
                preceded(comma, ubx_sat_status),
                preceded(comma, opt(uint32)),
                preceded(comma, opt(uint32)),
                preceded(comma, uint32),
                preceded(comma, uint32),
            )),
            |(id, status, azimuth, elevation, cno, lock_time)| UBXSatellite {
                id,
                status,
                azimuth,
                elevation,
                cno,
                lock_time,
            },
        ),
    )(input)
}

#[derive(Clone, Eq, Debug, PartialEq)]
pub struct UBXSatellites {
    pub satellites: Vec<UBXSatellite>,
}

pub(crate) fn ubx_03<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, UBXSatellites, E> {
    context(
        "UBX 03",
        all_consuming(map(
            preceded(
                preceded(
                    tag("PUBX"),
                    preceded(comma, preceded(tag("03"), preceded(comma, uint32))),
                ),
                many0(ubx_satellite),
            ),
            |satellites| UBXSatellites { satellites },
        )),
    )(input)
}

#[derive(Clone, Eq, Debug, PartialEq, Serialize)]
pub struct UBXTimePoll {}

#[derive(Clone, Debug, PartialEq)]
pub struct UBXTime {
    pub time: NaiveTime,
    pub date: NaiveDate,
    pub time_of_week: f32,
    pub week: u32,
    pub leap_seconds: u32,
    pub leap_second_default: bool,
    pub clock_bias: u32,
    pub clock_drift: f32,
    pub time_pulse_granularity: u32,
}

pub(crate) fn ubx_04<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, UBXTime, E> {
    context(
        "UBX 04",
        all_consuming(map(
            tuple((
                preceded(
                    tag("PUBX"),
                    preceded(comma, preceded(tag("04"), preceded(comma, time))),
                ),
                preceded(comma, date),
                preceded(comma, flt32),
                preceded(comma, uint32),
                preceded(comma, uint32),
                map(opt(char('D')), |c| c.is_some()),
                preceded(comma, uint32),
                preceded(comma, flt32),
                preceded(comma, terminated(uint32, comma)),
            )),
            |(
                time,
                date,
                time_of_week,
                week,
                leap_seconds,
                leap_second_default,
                clock_bias,
                clock_drift,
                time_pulse_granularity,
            )| UBXTime {
                time,
                date,
                time_of_week,
                week,
                leap_seconds,
                leap_second_default,
                clock_bias,
                clock_drift,
                time_pulse_granularity,
            },
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VLWData {
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

pub(crate) fn vlw<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VLWData, E> {
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
            )| VLWData {
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
pub struct VTGData {
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

pub(crate) fn vtg<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VTGData, E> {
    context(
        "VTG",
        all_consuming(map(
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
pub struct ZDAData {
    pub talker: Talker,
    pub time: NaiveTime,
    pub day: u32,
    pub month: u32,
    pub year: i32,
    pub local_tz_hour: i32,
    pub local_tz_minute: u32,
}

pub(crate) fn zda<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, ZDAData, E> {
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
            |(talker, time, day, month, year, local_tz_hour, local_tz_minute)| ZDAData {
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
