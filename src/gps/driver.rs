use crate::gps::Generic;
use crate::gps::UBloxNMEA;
use crate::gps::MKT;
use crate::nmea::NMEA;

use nom::error::ContextError;
use nom::error::FromExternalError;
use nom::error::ParseError;
use nom::IResult;

use std::num::ParseFloatError;
use std::num::ParseIntError;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Driver {
    UBloxNMEA(UBloxNMEA),
    MKT(MKT),
    Generic(Generic),
}

impl Driver {
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
        match self {
            Driver::Generic(d) => d.parse_private(input),
            Driver::MKT(d) => d.parse_private(input),
            Driver::UBloxNMEA(d) => d.parse_private(input),
        }
    }
}

impl Default for Driver {
    fn default() -> Self {
        Driver::Generic(Generic::default())
    }
}
