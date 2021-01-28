// For GlobalTop MKT devices

use crate::nmea::parser::*;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::sequence::*;
use nom::IResult;

use std::num::ParseIntError;

pub(crate) fn pmkt<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, MKTData, E> {
    context(
        "PMKT",
        alt((
            map(mkt_010, MKTData::SystemMessage),
            map(mkt_011, MKTData::TextMessage),
        )),
    )(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MKTData {
    SystemMessage(MKTSystemMessage),
    TextMessage(MKTTextMessage),
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

pub(crate) fn mkt_011<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
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
