use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::combinator::opt;
use nom::error::ParseError;
use nom::number::complete::be_u16;
use nom::sequence::delimited;
use nom::sequence::preceded;

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Command {
    Device,
    Devices,
    Poll,
    Version,
    Watch,
}

fn question<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("?")(input)
}

fn semicolon<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(";")(input)
}

fn equal<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("=")(input)
}

fn newline<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    preceded(opt(char('\r')), char('\n'))(input)
}

fn json_blob<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    let innards = take_while1(|c| c != '}');

    let blob = delimited(char('{'), innards, char('}'));

    blob(input)
}

fn device<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) =
        preceded(tag("?DEVICE"),
            preceded(opt(preceded(equal, json_blob)),
                preceded(semicolon, newline)))(input)?;

    Ok((input, Command::Device))
}

fn devices<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) =
        preceded(tag("?DEVICES"), preceded(semicolon, newline))(input)?;

    Ok((input, Command::Devices))
}

fn poll<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) =
        preceded(tag("?POLL"), preceded(semicolon, newline))(input)?;

    Ok((input, Command::Poll))
}

fn version<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) = preceded(tag("?VERSION;"), newline)(input)?;

    Ok((input, Command::Version))
}

fn watch<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    let (input, _) = preceded(tag("?WATCH;"), newline)(input)?;

    Ok((input, Command::Watch))
}

fn command<'a, E:ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Command, E> {
    alt((devices,
            device,
            poll,
            version,
            watch))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newline() {
        assert_eq!('\n', newline::<()>("\n").unwrap().1);
        assert_eq!('\n', newline::<()>("\r\n").unwrap().1);
    }

    #[test]
    fn test_device() {
        assert_eq!(Command::Device, device::<()>("?DEVICE;\n").unwrap().1);
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
        assert_eq!(Command::Watch, watch::<()>("?WATCH;\n").unwrap().1);
    }
}
