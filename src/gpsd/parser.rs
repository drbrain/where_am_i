use crate::gpsd::Device;
use crate::gpsd::Watch;
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
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Device(Option<Device>),
    Devices,
    Error(String),
    Poll,
    Version,
    Watch(Option<Watch>),
}

fn equal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("=")(input)
}

fn eol<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    preceded(char(';'), preceded(opt(char('\r')), char('\n')))(input)
}

fn json_blob<
    'a,
    T: Deserialize<'a>,
    E: ParseError<&'a str> + FromExternalError<&'a str, serde_json::Error>,
>(
    input: &'a str,
) -> IResult<&'a str, T, E> {
    let innards = take_while1(|c| c != '}');

    let blob = recognize(delimited(char('{'), innards, char('}')));

    map_res(blob, serde_json::from_str)(input)
}

fn device<'a, E: ParseError<&'a str> + FromExternalError<&'a str, serde_json::Error>>(
    input: &'a str,
) -> IResult<&'a str, Command, E> {
    let (input, device) = preceded(
        tag("?DEVICE"),
        terminated(opt(preceded(equal, json_blob::<Device, E>)), eol),
    )(input)?;

    Ok((input, Command::Device(device.into())))
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
        terminated(opt(preceded(equal, json_blob::<Watch, E>)), eol),
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

    #[test]
    fn test_eol() {
        assert_eq!('\n', eol::<()>(";\n").unwrap().1);
        assert_eq!('\n', eol::<()>(";\r\n").unwrap().1);
    }

    #[test]
    fn test_device() {
        assert_eq!(Command::Device(None), device::<()>("?DEVICE;\n").unwrap().1);

        let device_data = Device {
            path: Some("/dev/gps0".to_string()),
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

        let watch_data = Watch {
            device: Some("/dev/gps0".to_string()),
            enable: Some(true),
            ..Watch::default()
        };

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
