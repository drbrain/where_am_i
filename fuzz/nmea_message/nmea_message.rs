#![no_main]
use libfuzzer_sys::fuzz_target;
extern crate where_am_i;

use nom::error::VerboseError;
use nom::Err;

use where_am_i::nmea::message;
use where_am_i::nmea::NMEA;

fuzz_target!(|input: &[u8]| {
    if let Ok(i) = std::str::from_utf8(input) {
        let _ = parse(i);
    }
});

fn parse(input: &str) -> Result<NMEA, &str> {
    match message::<VerboseError<&str>>(input) {
        Ok((_rest, nmea)) => Ok(nmea),
        Err(Err::Incomplete(_)) => Err("incomplete"),
        Err(Err::Error(_)) => Err("error"),
        Err(Err::Failure(_)) => Err("failure"),
    }
}
