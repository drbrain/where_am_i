use crate::{
    gps::{Driver, Generic},
    nmea::{parser, sentence_parser, NMEA},
};
use nom::{error::VerboseErrorKind::Context, Err};
use std::time::Duration;

fn driver() -> Driver {
    Driver::Generic(Generic::default())
}

fn parse<'a>(input: &'a [u8]) -> NMEA {
    let driver = driver();

    parser::parse(input, &driver, timestamp()).unwrap().1
}

fn timestamp() -> Duration {
    Duration::from_secs(7)
}

#[test]
fn test_valid() {
    let parsed = parse(b"$GPGSV,3,2,10,09,32,158,27,05,22,295,19,27,22,044,31,13,20,312,19*74\r\n");

    let mut data = parser::gsv("GPGSV,3,2,10,09,32,158,27,05,22,295,19,27,22,044,31,13,20,312,19")
        .unwrap()
        .1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GSV(data), parsed);
}

#[test]
fn test_incomplete() {
    let driver = driver();
    let input = b"\r\n$EIGAQ,RMC*2B";

    match parser::parse(input, &driver, timestamp()) {
        Err(Err::Incomplete(nom::Needed::Size(needed))) => {
            assert_eq!(std::num::NonZeroUsize::new(1).unwrap(), needed)
        }
        _ => {
            panic!("Expected Incomplete");
        }
    }
}

#[test]
fn test_skip_garbage() {
    let parsed = parse(b"stuff*AA\r\n$EIGAQ,RMC*2B\r\n$");
    let mut data = parser::gaq("EIGAQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GAQ(data), parsed);

    let parsed = parse(b"\r\n$EIGAQ,RMC*2B\r\n");
    let mut data = parser::gaq("EIGAQ,RMC").unwrap().1;

    data.received = Some(timestamp());

    assert_eq!(NMEA::GAQ(data), parsed);
}

#[test]
fn test_garbage() {
    let input = b"$";
    let (input, count) = sentence_parser::garbage(input).unwrap();

    assert_eq!(0, count);
    assert_eq!(b"$", input);

    let input = b"x$";
    let (input, count) = sentence_parser::garbage(input).unwrap();

    assert_eq!(1, count);
    assert_eq!(b"$", input);

    let input = b"\r\n$";
    let (input, count) = sentence_parser::garbage(input).unwrap();

    assert_eq!(2, count);
    assert_eq!(b"$", input);

    let input = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx$";
    let (input, count) = sentence_parser::garbage(input).unwrap();

    assert_eq!(164, count);
    assert_eq!(b"$", input);

    let input = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx$";
    let result = sentence_parser::garbage(input);

    if let Err(Err::Failure(mut f)) = result {
        assert_eq!(Context("garbage"), f.errors.pop().unwrap().1);
    } else {
        assert!(false, "Garbage limit not reached");
    }
}
