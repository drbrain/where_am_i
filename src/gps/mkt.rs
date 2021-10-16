// For GlobalTop MKT devices

use crate::gps::add_message;
use crate::nmea::device::MessageSetting;
use crate::nmea::device::SerialCodec;
use crate::nmea::parser_util::*;
use crate::nmea::NMEA;

use futures_util::sink::SinkExt;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::sequence::*;
use nom::IResult;

use serde::Serialize;

use std::num::ParseIntError;

use tracing::debug;
use tracing::error;
use tracing::info;

pub const OUTPUT_MESSAGES: [&str; 7] = ["GGA", "GLL", "GSA", "GSV", "MCHN", "RMC", "VTG"];

#[derive(Clone, Default, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MKT {}

impl MKT {
    pub async fn configure(&self, serial: &mut SerialCodec, messages: Vec<MessageSetting>) {
        debug!("configuring MKT with sentences {:?}", messages);

        let mut set = MKTSetNMEAOutput::default();

        for message in &messages {
            let frequency: u32 = match message.enabled {
                true => 1,
                false => 0,
            };

            match message.id.as_str() {
                "GGA" => {
                    set.gga = frequency;
                }
                "GLL" => {
                    set.gll = frequency;
                }
                "GSA" => {
                    set.gsa = frequency;
                }
                "GSV" => {
                    set.gsv = frequency;
                }
                "MCHN" => {
                    set.mchn = frequency;
                }
                "RMC" => {
                    set.rmc = frequency;
                }
                "VTG" => {
                    set.vtg = frequency;
                }
                unknown => {
                    error!(
                        "MKT receivers don't support enabling {} sentences, ignored",
                        unknown
                    );
                }
            }
        }

        let summary = messages
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.id.clone())
            .collect::<Vec<String>>()
            .join(", ");

        match serial.send(set).await {
            Ok(_) => info!("enabling messages {}", summary),
            Err(e) => error!("unable to enable messages {}, {:?}", summary, e),
        }
    }

    pub fn message_settings(&self, messages: Vec<String>) -> Vec<MessageSetting> {
        let mut message_settings: Vec<MessageSetting> = vec![];

        if messages.is_empty() {
            for message in &OUTPUT_MESSAGES {
                add_message(&mut message_settings, message, true);
            }
        } else {
            for default in &OUTPUT_MESSAGES {
                let enabled = messages.contains(&default.to_string());

                add_message(&mut message_settings, &default.to_string(), enabled);
            }
        }

        message_settings
    }
    pub fn parse_private<
        'a,
        E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
    >(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, NMEA, E> {
        context(
            "PMKT",
            map(
                alt((
                    map(mkt_001, MKTData::Acknowledge),
                    map(mkt_010, MKTData::SystemMessage),
                    map(mkt_011, MKTData::TextMessage),
                )),
                NMEA::PMKT,
            ),
        )(input)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MKTData {
    Acknowledge(MKTAcknowledge),
    SystemMessage(MKTSystemMessage),
    TextMessage(MKTTextMessage),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MKTAcknowledge {
    Invalid,
    Unsupported,
    Failed,
    Succeeded,
    Unhandled(u32),
}

pub(crate) fn mkt_001<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, MKTAcknowledge, E> {
    let result = parse_message(
        "MKT 001",
        preceded(preceded(tag("PMTK001"), comma), uint32),
        |m| match m {
            0 => MKTAcknowledge::Invalid,
            1 => MKTAcknowledge::Unsupported,
            2 => MKTAcknowledge::Failed,
            3 => MKTAcknowledge::Succeeded,
            u => MKTAcknowledge::Unhandled(u),
        },
    )(input);

    if let Ok((_, ref acknowledgement)) = result {
        match acknowledgement {
            MKTAcknowledge::Invalid => {
                error!("Invalid PMKT command");
            }
            MKTAcknowledge::Unsupported => {
                error!("Unsupported PMKT command");
            }
            MKTAcknowledge::Failed => {
                error!("Failed (but valid) PMKT command");
            }
            MKTAcknowledge::Succeeded => {
                info!("Successful PMKT command");
            }
            MKTAcknowledge::Unhandled(u) => {
                error!("Unhandled PMKT acknowledgement {}", u);
            }
        };
    }

    result
}

#[derive(Clone, Debug, PartialEq)]
pub enum MKTSystemMessage {
    Unknown,
    Startup,
    ExtendedPredictionOrbit,
    Normal,
    Unhandled(u32),
}

pub(crate) fn mkt_010<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, MKTSystemMessage, E> {
    parse_message(
        "MKT 010",
        preceded(preceded(tag("PMTK010"), comma), uint32),
        |m| match m {
            0 => MKTSystemMessage::Unknown,
            1 => MKTSystemMessage::Startup,
            2 => MKTSystemMessage::ExtendedPredictionOrbit,
            3 => MKTSystemMessage::Normal,
            u => MKTSystemMessage::Unhandled(u),
        },
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MKTTextMessage {
    pub message: String,
}

pub(crate) fn mkt_011<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, MKTTextMessage, E> {
    parse_message(
        "MKT 011",
        preceded(preceded(tag("PMTK011"), comma), rest),
        |m| MKTTextMessage {
            message: m.to_string(),
        },
    )(input)
}

#[derive(Default, Clone, Eq, Debug, PartialEq, Serialize)]
pub struct MKTSetNMEAOutput {
    pub gll: u32,
    pub rmc: u32,
    pub vtg: u32,
    pub gga: u32,
    pub gsa: u32,
    pub gsv: u32,
    _6: u32,
    _7: u32,
    _8: u32,
    _9: u32,
    _10: u32,
    _11: u32,
    _12: u32,
    _13: u32,
    _14: u32,
    _15: u32,
    _16: u32,
    _17: u32,
    pub mchn: u32,
}
