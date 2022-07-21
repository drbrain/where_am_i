use crate::nmea::parser::Result;
use chrono::naive::{NaiveDate, NaiveTime};
use nom::{
    branch::alt,
    bytes::complete::*,
    character::complete::*,
    combinator::*,
    error::context,
    number::complete::recognize_float,
    sequence::{preceded, terminated, tuple},
};

pub(crate) fn any<'a>(input: &'a str) -> Result<&'a str, String> {
    map(take_while(|c| c != ','), |m: &str| m.to_string())(input)
}

pub(crate) fn comma<'a>(input: &'a str) -> Result<&'a str, &'a str> {
    tag(",")(input)
}

pub(crate) fn date<'a>(input: &'a str) -> Result<&'a str, NaiveDate> {
    map_opt(
        tuple((two_digit, two_digit, two_digit_i)),
        |(day, month, year)| NaiveDate::from_ymd_opt(year, month, day),
    )(input)
}

pub(crate) fn dot<'a>(input: &'a str) -> Result<&'a str, &'a str> {
    tag(".")(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum EastWest {
    East,
    West,
}

pub(crate) fn east_west<'a>(input: &'a str) -> Result<&'a str, EastWest> {
    map(one_of("EW"), |ew| match ew {
        'E' => EastWest::East,
        'W' => EastWest::West,
        _ => panic!("Unknown direction {:?}", ew),
    })(input)
}

pub(crate) fn flt32<'a>(input: &'a str) -> Result<&'a str, f32> {
    map_res(recognize_float, |s: &str| s.parse())(input)
}

pub(crate) fn is_digit(chr: char) -> bool {
    chr.is_ascii_digit()
}

pub(crate) fn int32<'a>(input: &'a str) -> Result<&'a str, i32> {
    map_res(
        recognize(preceded(opt(char('-')), take_while(is_digit))),
        |s: &str| s.parse(),
    )(input)
}

pub(crate) fn is_upper_alphanum(chr: char) -> bool {
    chr.is_ascii_uppercase() || chr.is_ascii_digit()
}

pub(crate) fn lat<'a>(input: &'a str) -> Result<&'a str, f32> {
    map(tuple((two_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

pub(crate) fn lon<'a>(input: &'a str) -> Result<&'a str, f32> {
    map(tuple((three_digit, flt32)), |(d, m)| d as f32 + m / 60.0)(input)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatLon {
    pub latitude: f32,
    pub longitude: f32,
}

pub(crate) fn latlon<'a>(input: &'a str) -> Result<&'a str, Option<LatLon>> {
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

pub(crate) fn north_south<'a>(input: &'a str) -> Result<&'a str, NorthSouth> {
    map(one_of("NS"), |ns| match ns {
        'N' => NorthSouth::North,
        'S' => NorthSouth::South,
        _ => panic!("Unhandled direction {:?}", ns),
    })(input)
}

pub fn parse_message<I, O1, O2, F, G>(
    message: &'static str,
    first: F,
    second: G,
) -> impl FnMut(I) -> Result<I, O2>
where
    F: nom::Parser<I, O1, nom::error::VerboseError<I>>,
    G: FnMut(O1) -> O2,
    I: Clone + nom::InputLength,
{
    context(message, all_consuming(map(first, second)))
}

pub(crate) fn three_digit<'a>(input: &'a str) -> Result<&'a str, u32> {
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

pub(crate) fn time<'a>(input: &'a str) -> Result<&'a str, NaiveTime> {
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

pub(crate) fn two_digit<'a>(input: &'a str) -> Result<&'a str, u32> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn two_digit_i<'a>(input: &'a str) -> Result<&'a str, i32> {
    map_res(take_while_m_n(2, 2, is_digit), |i: &str| i.parse())(input)
}

pub(crate) fn uint32<'a>(input: &'a str) -> Result<&'a str, u32> {
    map_res(take_while(is_digit), |s: &str| s.parse())(input)
}
