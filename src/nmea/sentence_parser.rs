use crate::nmea::parser::{ChecksumMismatch, Result};
use chrono::NaiveDateTime;
use nom::{
    bytes::streaming::{tag, take_while_m_n},
    character::is_hex_digit,
    combinator::{cut, map, opt, peek, recognize},
    error::context,
    sequence::{delimited, preceded, terminated, tuple},
    Err, Needed,
};
use std::convert::TryInto;
use std::time::Duration;
use tracing::{error, trace};

#[derive(Debug)]
pub enum NMEASentence<'a> {
    InvalidChecksum(ChecksumMismatch),
    ParseError(String),
    Valid(&'a str),
}

pub(crate) fn parse_sentence<'a>(
    input: &'a [u8],
    received: Duration,
) -> Result<&'a [u8], NMEASentence<'a>> {
    let result = delimited(
        preceded(garbage, tag(b"$")),
        tuple((terminated(non_star, star), checksum)),
        terminated(opt(tag(b"\r")), tag(b"\n")),
    )(input);

    let (input, (data, given)) = match result {
        Err(Err::Incomplete(_)) => {
            return Err(result.err().unwrap());
        }
        Err(e) => return parse_error(input, e),
        Ok(t) => t,
    };

    let calculated = data.iter().fold(0, |c, b| c ^ b);
    let data = std::str::from_utf8(data).unwrap();

    let result = if given == calculated {
        trace!(
            "received {:?} parsing \"{}\" (checksum OK), {} bytes remaining",
            NaiveDateTime::from_timestamp(
                received.as_secs().try_into().unwrap_or(0),
                received.subsec_nanos()
            ),
            data,
            input.len()
        );

        NMEASentence::Valid(data)
    } else {
        trace!(
            "invalid checksum for \"{}\" ({} != {})",
            data,
            given,
            calculated
        );

        let message = String::from(data);

        NMEASentence::InvalidChecksum(ChecksumMismatch {
            message,
            given,
            calculated,
        })
    };

    Ok((input, result))
}

fn parse_error<'a>(
    input: &'a [u8],
    e: nom::Err<nom::error::VerboseError<&'a [u8]>>,
) -> Result<&'a [u8], NMEASentence<'a>> {
    let error = match e {
        Err::Incomplete(Needed::Size(n)) => format!("incomplete, need {}", n),
        Err::Incomplete(Needed::Unknown) => "incomplete".to_string(),
        Err::Error(_) => "(recoverable)".to_string(),
        Err::Failure(_) => "(failure)".to_string(),
    };

    match std::str::from_utf8(input) {
        Ok(i) => {
            error!("Error {:?} parsing {}", error, i);

            Ok((b"", NMEASentence::ParseError(String::from(i))))
        }
        Err(e) => {
            error!("Some error parsing: {:?} (invalid UTF-8: {})", input, e);

            let next_invalid = e.valid_up_to() + e.error_len().unwrap_or(1);

            Ok((
                &input[next_invalid..],
                NMEASentence::ParseError(String::from("Invalid UTF-8")),
            ))
        }
    }
}

pub(crate) fn garbage<'a>(input: &'a [u8]) -> Result<&'a [u8], usize> {
    context(
        "garbage",
        cut(terminated(
            map(take_while_m_n(0, 164, |c| c != b'$'), |g: &[u8]| g.len()),
            peek(tag(b"$")),
        )),
    )(input)
}

pub(crate) fn non_star<'a>(input: &'a [u8]) -> Result<&'a [u8], &'a [u8]> {
    use nom::bytes::streaming::take_till;

    recognize(take_till(|c| c == b'*'))(input)
}

pub(crate) fn star<'a>(input: &'a [u8]) -> Result<&'a [u8], &'a [u8]> {
    tag(b"*")(input)
}

pub(crate) fn checksum<'a>(input: &'a [u8]) -> Result<&'a [u8], u8> {
    map(recognize(take_while_m_n(2, 2, is_hex_digit)), |c| {
        u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap()
    })(input)
}
