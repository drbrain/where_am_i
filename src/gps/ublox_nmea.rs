// For UBlox ZED-F9P devices

use chrono::naive::NaiveDate;
use chrono::naive::NaiveTime;

use crate::nmea::device::MessageSetting;
use crate::nmea::device::SerialCodec;
use crate::nmea::parser::*;
use crate::nmea::NMEA;

use futures_util::sink::SinkExt;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_while_m_n;
use nom::character::complete::char;
use nom::character::complete::one_of;
use nom::combinator::all_consuming;
use nom::combinator::map;
use nom::combinator::opt;
use nom::error::context;
use nom::error::ContextError;
use nom::error::FromExternalError;
use nom::error::ParseError;
use nom::multi::many0;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::sequence::tuple;
use nom::IResult;

use serde::ser::SerializeStruct;
use serde::ser::Serializer;
use serde::Serialize;

use std::num::ParseFloatError;
use std::num::ParseIntError;

use tracing::error;
use tracing::info;

pub const UBX_OUTPUT_MESSAGES: [&str; 15] = [
    "DTM", "GBS", "GGA", "GLL", "GNS", "GRS", "GSA", "GST", "GSV", "RLM", "RMC", "TXT", "VLW",
    "VTG", "ZDA",
];

#[derive(Clone, Default, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UBloxNMEA {}

impl UBloxNMEA {
    pub async fn configure(&self, serial: &mut SerialCodec, messages: Vec<MessageSetting>) {
        for message in messages {
            let rate = rate_for(message.id.clone(), message.enabled);

            match serial.send(rate).await {
                Ok(_) => info!("setting {} to {}", message.id, message.enabled),
                Err(e) => error!(
                    "unable to set {} to {}: {:?}",
                    message.id, message.enabled, e
                ),
            }
        }
    }

    pub fn parse_private<
        'a,
        E: ParseError<&'a str>
            + ContextError<&'a str>
            + FromExternalError<&'a str, ParseFloatError>
            + FromExternalError<&'a str, ParseIntError>,
    >(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, NMEA, E> {
        context(
            "PUBX",
            map(
                alt((
                    map(ubx_00, UBXData::Position),
                    map(ubx_03, UBXData::Satellites),
                    map(ubx_04, UBXData::Time),
                )),
                NMEA::PUBX,
            ),
        )(input)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UBXData {
    Position(UBXPosition),
    Satellites(UBXSatellites),
    Time(UBXTime),
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

fn rate_for(msg_id: String, enabled: bool) -> UBXRate {
    let rus1 = if enabled { 1 } else { 0 };

    UBXRate {
        message: msg_id,
        rddc: 0,
        rus1,
        rus2: 0,
        rusb: 0,
        rspi: 0,
        reserved: 0,
    }
}

#[derive(Clone, Eq, Debug, PartialEq)]
pub enum UBXPort {
    I2C = 0,
    USART1 = 1,
    USART2 = 2,
    USB = 3,
    SPI = 4,
}

impl Serialize for UBXPort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("UBXPort", 1)?;
        let value: u32 = match self {
            UBXPort::I2C => 0,
            UBXPort::USART1 => 1,
            UBXPort::USART2 => 2,
            UBXPort::USB => 3,
            UBXPort::SPI => 4,
        };

        state.serialize_field("no comma", &value)?;
        state.end()
    }
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

impl Serialize for UBXPortMask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("UBXPortMask", 1)?;
        state.serialize_field("no comma", &self.bits())?;
        state.end()
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

pub(crate) fn ubx_nav_stat<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
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

pub(crate) fn ubx_00<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
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

pub(crate) fn ubx_satellite<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
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

pub(crate) fn ubx_03<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
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

pub(crate) fn ubx_04<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, UBXTime, E> {
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
