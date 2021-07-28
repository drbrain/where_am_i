use crate::gps::Driver;
use crate::gps::Generic;
use crate::nmea::parser;
use crate::nmea::sentence_parser;
use crate::nmea::NMEA;

use nom::error::VerboseError;
use nom::error::VerboseErrorKind::Context;
use nom::Err;

use std::time::Duration;

type VE<'a> = VerboseError<&'a str>;
type VEb<'a> = VerboseError<&'a [u8]>;

fn driver() -> Driver {
    Driver::Generic(Generic::default())
}

fn parse<'a>(input: &'a [u8]) -> NMEA {
    let driver = driver();

    parser::parse::<VEb>(input, &driver, timestamp()).unwrap().1
}

fn timestamp() -> Duration {
    Duration::from_secs(7)
}

#[test]
fn test_skip_garbage() {
    let parsed = parse(b"stuff*AA\r\n$EIGAQ,RMC*2B\r\n$");
    let mut data = parser::gaq::<VE>("EIGAQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GAQ(data), parsed);
}

#[test]
fn test_garbage() {
    let input = b"$";
    let (input, count) = sentence_parser::garbage::<VEb>(input).unwrap();

    assert_eq!(0, count);
    assert_eq!(b"$", input);

    let input = b"x$";
    let (input, count) = sentence_parser::garbage::<VEb>(input).unwrap();

    assert_eq!(1, count);
    assert_eq!(b"$", input);

    let input = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx$";
    let (input, count) = sentence_parser::garbage::<VEb>(input).unwrap();

    assert_eq!(164, count);
    assert_eq!(b"$", input);

    let input = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx$";
    let result = sentence_parser::garbage::<VEb>(input);

    if let Err(Err::Failure(mut f)) = result {
        assert_eq!(Context("garbage"), f.errors.pop().unwrap().1);
    } else {
        assert!(false, "Garbage limit not reached");
    }
}
