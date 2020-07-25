use json;
use json::JsonValue;

use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::combinator::opt;
use nom::combinator::map_opt;
use nom::combinator::map_res;
use nom::combinator::recognize;
use nom::combinator::value;
use nom::error::ParseError;
use nom::number::complete::be_u16;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::terminated;

pub fn length_value(input: &[u8]) -> IResult<&[u8],&[u8]> {
    let (input, length) = be_u16(input)?;
    take(length)(input)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Needed {
  Unknown,
  Size(u32)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Err<E> {
  Incomplete(Needed),
  Error(E),
  Failure(E),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DeviceData {
    pub path: Option<String>,
    pub bps: Option<u32>,
    pub parity: Option<String>,
    pub stopbits: Option<u32>,
    pub native: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WatchData {
    pub enable: bool,
    pub json: bool,
    pub raw: u32,
    pub scaled: bool,
    pub split24: bool,
    pub pps: bool,
    pub device: Option<String>,
    pub remote: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Device(Option<DeviceData>),
    Devices,
    Error,
    Poll,
    Version,
    Watch(Option<WatchData>),
}

fn json_to_string(input: &JsonValue) -> Option<String> {
    if input.is_null() {
        None
    } else {
        input.as_str().map_or(None, |v| Some(v.to_string()))
    }
}

fn equal<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("=")(input)
}

fn eol<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    preceded(char(';'), preceded(opt(char('\r')), char('\n')))(input)
}

fn json_blob<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, JsonValue, E> {
    let innards = take_while1(|c| c != '}');

    let blob = recognize(delimited(char('{'), innards, char('}')));

    map_res(blob, |j| json::parse(j))(input)
}

fn device<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, json) =
        preceded(tag("?DEVICE"),
            terminated(opt(preceded(equal, json_blob)),
                eol))(input)?;

    let device_data = match json {
        Some(j) =>
            Some(DeviceData {
                path: json_to_string(&j["path"]),
                bps: j["bps"].as_u32(),
                parity: json_to_string(&j["parity"]),
                stopbits: j["stopbits"].as_u32(),
                native: j["native"].as_u32(),
            }),
        None => None,
    };

    Ok((input, Command::Device(device_data)))
}

fn devices<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) =
        preceded(tag("?DEVICES"), eol)(input)?;

    Ok((input, Command::Devices))
}

fn poll<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) =
        preceded(tag("?POLL"), eol)(input)?;

    Ok((input, Command::Poll))
}

fn version<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) = preceded(tag("?VERSION"), eol)(input)?;

    Ok((input, Command::Version))
}

fn watch<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, json) =
        preceded(tag("?WATCH"),
            terminated(opt(preceded(equal, json_blob)),
                eol))(input)?;

    let watch_data = match json {
        Some(j) =>
            Some(WatchData {
                enable: j["enable"].as_bool().unwrap_or(false),
                json: j["json"].as_bool().unwrap_or(false),
                raw: 0,
                scaled: j["scaled"].as_bool().unwrap_or(false),
                split24: j["split24"].as_bool().unwrap_or(false),
                pps: j["pps"].as_bool().unwrap_or(false),
                device: json_to_string(&j["device"]),
                remote: json_to_string(&j["remote"]),
        }),
        None => None,
    };

    Ok((input, Command::Watch(watch_data)))
}

fn command<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (_, command) =
        alt((devices,
            device,
            poll,
            version,
            watch))(input)?;

    Ok((input, command))
}

fn parse(input: &str) -> Command {
    match command::<()>(input) {
        Ok((_, c)) => c,
        Err(_)      => Command::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eol() {
        assert_eq!('\n', eol::<()>(";\n").unwrap().1);
        assert_eq!('\n', eol::<()>(";\r\n").unwrap().1);
    }

    #[test]
    fn test_json_blob() {
        let expected = json::parse("{\"hello\":true}").unwrap();

        assert_eq!(expected, json_blob::<()>("{\"hello\":true}").unwrap().1);
    }

    #[test]
    fn test_device() {
        assert_eq!(Command::Device(None), device::<()>("?DEVICE;\n").unwrap().1);

        let device_data = DeviceData {
            path: Some("/dev/gps0".to_string()),
            bps: Some(38400),
            parity: None,
            stopbits: None,
            native: None,
        };

        assert_eq!(Command::Device(Some(device_data)), device::<()>("?DEVICE={\"path\":\"/dev/gps0\",\"bps\":38400};\n").unwrap().1);
    }

    #[test]
    fn test_devices() {
        assert_eq!(Command::Devices, devices::<()>("?DEVICES;\n").unwrap().1);
    }

    #[test]
    fn test_poll() {
        assert_eq!(Command::Poll, poll::<()>("?POLL;\n").unwrap().1);
    }

    #[test]
    fn test_version() {
        assert_eq!(Command::Version, version::<()>("?VERSION;\n").unwrap().1);
    }

    #[test]
    fn test_watch() {
        assert_eq!(Command::Watch(None), watch::<()>("?WATCH;\n").unwrap().1);

        let watch_data = WatchData {
            enable: true,
            json: false,
            raw: 0,
            scaled: false,
            split24: false,
            pps: false,
            device: Some("/dev/gps0".to_string()),
            remote: None,
        };

        assert_eq!(Command::Watch(Some(watch_data)), watch::<()>("?WATCH={\"device\":\"/dev/gps0\",\"enable\":true};\n").unwrap().1);
    }

    #[test]
    fn test_command() {
        assert_eq!(Command::Device(None), command::<()>("?DEVICE;\n").unwrap().1);
        assert_eq!(Command::Devices, command::<()>("?DEVICES;\n").unwrap().1);
        assert_eq!(Command::Poll, command::<()>("?POLL;\n").unwrap().1);
        assert_eq!(Command::Version, command::<()>("?VERSION;\n").unwrap().1);
        assert_eq!(Command::Watch(None), command::<()>("?WATCH;\n").unwrap().1);
    }

    #[test]
    fn test_parse() {
        assert_eq!(Command::Watch(None), parse("?WATCH;\n"));
        assert_eq!(Command::Error, parse("garbage\n"));
    }
}
