use chrono::naive::NaiveDate;
use chrono::naive::NaiveTime;

use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::ContextError;
use nom::error::FromExternalError;
use nom::error::ParseError;
use nom::number::complete::recognize_float;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::sequence::tuple;
use nom::IResult;

use std::num::ParseFloatError;
use std::num::ParseIntError;

pub(crate) fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    map(take_while(|c| c != ','), |m: &str| m.to_string())(input)
}

pub(crate) fn comma<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(",")(input)
}

pub(crate) fn date<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, NaiveDate, E> {
    map_opt(
        tuple((two_digit, two_digit, two_digit_i)),
        |(day, month, year)| NaiveDate::from_ymd_opt(year, month, day),
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

pub(crate) fn flt32<
    'a,
    E: ParseError<&'a str> + ContextError<&'a str> + FromExternalError<&'a str, ParseFloatError>,
>(
    input: &'a str,
) -> IResult<&'a str, f32, E> {
    map_res(recognize_float, |s: &str| s.parse())(input)
}

pub(crate) fn is_digit(chr: char) -> bool {
    chr.is_ascii_digit()
}

pub(crate) fn int32<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, i32, E> {
    map_res(
        recognize(preceded(opt(char('-')), take_while(is_digit))),
        |s: &str| s.parse(),
    )(input)
}

pub(crate) fn is_upper_alphanum(chr: char) -> bool {
    chr.is_ascii_uppercase() || chr.is_ascii_digit()
}

pub(crate) fn lat<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ParseFloatError>,
>(
    input: &'a str,
) -> IResult<&'a str, f32, E> {
    map(tuple((two_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

pub(crate) fn lon<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, f32, E> {
    map(tuple((three_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatLon {
    pub latitude: f32,
    pub longitude: f32,
}

pub(crate) fn latlon<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
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
pub enum NorthSouth {
    North,
    South,
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

pub(crate) fn three_digit<
    'a,
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(3, 3, is_digit), |i: &str| i.parse())(input)
}

enum TimeResolution {
    Centisecond(u32),
    Millisecond(u32),
}

// Parses time without subseconds: 010203
//
// with centiseconds: 010203.45
//
// with milliseconds: 010203.456

pub(crate) fn time<
    'a,
    E: ParseError<&'a str>
        + ContextError<&'a str>
        + FromExternalError<&'a str, ParseFloatError>
        + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, NaiveTime, E> {
    map_opt(
        tuple((
            two_digit,
            two_digit,
            two_digit,
            opt(preceded(
                dot,
                alt((
                    map(three_digit, TimeResolution::Millisecond),
                    map(two_digit, TimeResolution::Centisecond),
                )),
            )),
        )),
        |(hour, minute, second, subsec)| match subsec {
            Some(TimeResolution::Centisecond(subsec)) => {
                NaiveTime::from_hms_milli_opt(hour, minute, second, subsec * 10)
            }
            Some(TimeResolution::Millisecond(subsec)) => {
                NaiveTime::from_hms_milli_opt(hour, minute, second, subsec)
            }
            None => NaiveTime::from_hms_milli_opt(hour, minute, second, 0),
        },
    )(input)
}

pub(crate) fn two_digit<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, u32, E> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn two_digit_i<
    'a,
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>,
>(
    input: &'a str,
) -> IResult<&'a str, i32, E> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn uint32<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    input: &'a str,
) -> IResult<&'a str, u32, E> {
    map_res(take_while(is_digit), |s: &str| s.parse())(input)
}
