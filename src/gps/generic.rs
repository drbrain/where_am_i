use crate::nmea::NMEA;
use nom::{
    combinator::{map, rest},
    error::context,
    IResult,
};

#[derive(Clone, Default, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Generic {}

impl Generic {
    pub fn parse_private<'a>(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, NMEA, nom::error::VerboseError<&'a str>> {
        context(
            "private message",
            map(rest, |m: &str| NMEA::Unsupported(m.to_string())),
        )(input)
    }
}
