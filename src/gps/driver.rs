use crate::gps::Generic;
use crate::gps::UBloxNMEA;
use crate::gps::MKT;
use crate::nmea::MessageSetting;
use crate::nmea::SerialCodec;
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
    pub async fn configure(&self, serial: &mut SerialCodec, messages: &Vec<MessageSetting>) {
        match self {
            Driver::Generic(_) => (),
            Driver::MKT(d) => d.configure(serial, messages).await,
            Driver::UBloxNMEA(d) => d.configure(serial, messages).await,
        }
    }

    pub fn message_settings(&self, messages: &Vec<String>) -> Vec<MessageSetting> {
        match self {
            Driver::Generic(_) => vec![],
            Driver::MKT(d) => d.message_settings(messages),
            Driver::UBloxNMEA(d) => d.message_settings(messages),
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

pub fn add_message(message_settings: &mut Vec<MessageSetting>, message: &str, enabled: bool) {
    let setting = MessageSetting {
        id: message.to_string(),
        enabled,
    };

    message_settings.push(setting);
}
