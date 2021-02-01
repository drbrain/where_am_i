use crate::nmea::NMEA;

use nom::combinator::map;
use nom::combinator::rest;
use nom::error::context;
use nom::error::ContextError;
use nom::error::ParseError;
use nom::IResult;

#[derive(Clone, Default, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Generic {}

impl Generic {
    pub fn parse_private<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, NMEA, E> {
        context(
            "private message",
            map(rest, |m: &str| NMEA::Unsupported(m.to_string())),
        )(input)
    }
}
