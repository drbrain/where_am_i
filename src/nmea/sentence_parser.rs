use chrono::NaiveDateTime;

use crate::nmea::parser::ChecksumMismatch;

use nom::bytes::complete::tag;
use nom::combinator::cut;
use nom::combinator::map;
use nom::combinator::opt;
use nom::combinator::peek;
use nom::combinator::recognize;
use nom::error::context;
use nom::error::ContextError;
use nom::error::ParseError;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::sequence::tuple;
use nom::Err;
use nom::IResult;
use nom::Needed;

use std::convert::TryInto;
use std::time::Duration;

use tracing::error;
use tracing::trace;

#[derive(Debug)]
pub enum NMEASentence<'a> {
    InvalidChecksum(ChecksumMismatch),
    ParseError(String),
    Valid(&'a str),
}

pub(crate) fn parse_sentence<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
    input: &'a [u8],
    received: Duration,
) -> IResult<&'a [u8], NMEASentence<'a>, E> {
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

fn parse_error<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
    e: nom::Err<E>,
) -> IResult<&'a [u8], NMEASentence, E> {
    let error = match e {
        Err::Incomplete(Needed::Size(n)) => format!("incomplete, need {}", n),
        Err::Incomplete(Needed::Unknown) => format!("incomplete"),
        Err::Error(_) => format!("(recoverable)"),
        Err::Failure(_) => format!("(failure)"),
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

pub(crate) fn garbage<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], usize, E> {
    use nom::bytes::streaming::tag;
    use nom::bytes::streaming::take_while_m_n;

    context(
        "garbage",
        cut(terminated(
            map(take_while_m_n(0, 164, |c| c != b'$'), |g: &[u8]| g.len()),
            peek(tag(b"$")),
        )),
    )(input)
}

pub(crate) fn non_star<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], &'a [u8], E> {
    use nom::bytes::streaming::take_till;

    recognize(take_till(|c| c == b'*'))(input)
}

pub(crate) fn star<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E> {
    use nom::bytes::streaming::tag;

    tag(b"*")(input)
}

pub(crate) fn checksum<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], u8, E> {
    use nom::bytes::streaming::take_while_m_n;
    use nom::character::is_hex_digit;

    map(recognize(take_while_m_n(2, 2, is_hex_digit)), |c| {
        u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap()
    })(input)
}
