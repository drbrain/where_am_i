use serde_json::Value;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::combinator::map_res;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::error::FromExternalError;
use nom::error::ParseError;
use nom::error::VerboseError;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::IResult;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DeviceData {
    pub path: Option<String>,
    pub bps: Option<u64>,
    pub parity: Option<String>,
    pub stopbits: Option<u64>,
    pub native: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Device(Option<DeviceData>),
    Devices,
    Error(String),
    Poll,
    Version,
    Watch(Option<Value>),
}

pub fn json_to_string(input: &Value) -> Option<String> {
    if input.is_null() {
        None
    } else {
        input.as_str().map(|v| v.to_string())
    }
}

fn equal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("=")(input)
}

fn eol<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    preceded(char(';'), preceded(opt(char('\r')), char('\n')))(input)
}

fn json_blob<'a, E: ParseError<&'a str> + FromExternalError<&'a str, serde_json::Error>>(
    input: &'a str,
) -> IResult<&'a str, Value, E> {
    let innards = take_while1(|c| c != '}');

    let blob = recognize(delimited(char('{'), innards, char('}')));

    map_res(blob, serde_json::from_str)(input)
}

fn device<'a, E: ParseError<&'a str> + FromExternalError<&'a str, serde_json::Error>>(
    input: &'a str,
) -> IResult<&'a str, Command, E> {
    let (input, json) = preceded(
        tag("?DEVICE"),
        terminated(opt(preceded(equal, json_blob)), eol),
    )(input)?;

    let device_data = json.map(|j| DeviceData {
        path: json_to_string(&j["path"]),
        bps: j["bps"].as_u64(),
        parity: json_to_string(&j["parity"]),
        stopbits: j["stopbits"].as_u64(),
        native: j["native"].as_u64(),
    });

    Ok((input, Command::Device(device_data)))
}

fn devices<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) = preceded(tag("?DEVICES"), eol)(input)?;

    Ok((input, Command::Devices))
}

fn poll<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) = preceded(tag("?POLL"), eol)(input)?;

    Ok((input, Command::Poll))
}

fn version<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) = preceded(tag("?VERSION"), eol)(input)?;

    Ok((input, Command::Version))
}

fn watch<'a, E: ParseError<&'a str> + FromExternalError<&'a str, serde_json::Error>>(
    input: &'a str,
) -> IResult<&'a str, Command, E> {
    let (input, json) = preceded(
        tag("?WATCH"),
        terminated(opt(preceded(equal, json_blob)), eol),
    )(input)?;

    Ok((input, Command::Watch(json)))
}

fn command<'a, E: ParseError<&'a str> + FromExternalError<&'a str, serde_json::Error>>(
    input: &'a str,
) -> IResult<&'a str, Command, E> {
    let (_, command) = alt((devices, device, poll, version, watch))(input)?;

    Ok((input, command))
}

pub fn parse(input: &str) -> Command {
    match command::<VerboseError<&str>>(input) {
        Ok((_, c)) => c,
        Err(e) => Command::Error(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_eol() {
        assert_eq!('\n', eol::<()>(";\n").unwrap().1);
        assert_eq!('\n', eol::<()>(";\r\n").unwrap().1);
    }

    #[test]
    fn test_json_blob() {
        let expected: Value = serde_json::from_str("{\"hello\":true}").unwrap();

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

        assert_eq!(
            Command::Device(Some(device_data)),
            device::<()>("?DEVICE={\"path\":\"/dev/gps0\",\"bps\":38400};\n")
                .unwrap()
                .1
        );
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

        let watch_data = json!({
            "device": "/dev/gps0",
            "enable": true,
        });

        assert_eq!(
            Command::Watch(Some(watch_data)),
            watch::<()>("?WATCH={\"device\":\"/dev/gps0\",\"enable\":true};\n")
                .unwrap()
                .1
        );
    }

    #[test]
    fn test_command() {
        assert_eq!(
            Command::Device(None),
            command::<()>("?DEVICE;\n").unwrap().1
        );
        assert_eq!(Command::Devices, command::<()>("?DEVICES;\n").unwrap().1);
        assert_eq!(Command::Poll, command::<()>("?POLL;\n").unwrap().1);
        assert_eq!(Command::Version, command::<()>("?VERSION;\n").unwrap().1);
        assert_eq!(Command::Watch(None), command::<()>("?WATCH;\n").unwrap().1);
    }

    #[test]
    fn test_parse() {
        assert_eq!(Command::Watch(None), parse("?WATCH;\n"));
        assert_eq!(Command::Error("Parsing Error: VerboseError { errors: [(\"garbage\\n\", Nom(Tag)), (\"garbage\\n\", Nom(Alt))] }".to_string()), parse("garbage\n"));
    }
}
