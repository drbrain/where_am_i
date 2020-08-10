use crate::nmea::parser::parse;
use crate::nmea::parser::NMEA;
use crate::nmea::ser;

use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;

use nom::error::VerboseError;
use nom::Err;

use serde::Serialize;

use std::fmt;
use std::io;

use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

use tracing::debug;

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Codec {}

type VE<'a> = VerboseError<&'a [u8]>;

impl Decoder for Codec {
    type Item = NMEA;
    type Error = CodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<NMEA>, CodecError> {
        let bytes = buf.to_bytes();
        let input = bytes.bytes();

        match parse::<VE>(input) {
            Ok((input, nmea)) => {
                buf.extend_from_slice(&Bytes::copy_from_slice(input));
                Ok(Some(nmea))
            }
            Err(Err::Incomplete(_)) => {
                buf.extend_from_slice(&Bytes::copy_from_slice(input));
                Ok(None)
            }
            Err(Err::Error(_)) => panic!("impossible error!"),
            Err(Err::Failure(_)) => panic!("impossible failure!"),
        }
    }
}

impl<T> Encoder<T> for Codec
where
    T: Serialize,
{
    type Error = CodecError;

    fn encode(&mut self, nmea: T, buf: &mut BytesMut) -> Result<(), CodecError> {
        let message = match ser::to_string(&nmea) {
            Ok(m) => m,
            Err(_) => return Err(CodecError::InternalError),
        };

        let checksum = message.bytes().fold(0, |c, b| c ^ b);
        let line = format!("${}*{:02X}\r\n", message, checksum);

        debug!("sending serial message: {:?}", line);

        buf.reserve(line.len());
        buf.put(line.as_bytes());

        Ok(())
    }
}

#[derive(Debug)]
pub enum CodecError {
    InternalError,
    UnrecognizedRequest,
    Io(io::Error),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::InternalError => write!(f, "internal error"),
            CodecError::UnrecognizedRequest => write!(f, "unrecognized request"),
            CodecError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<io::Error> for CodecError {
    fn from(e: io::Error) -> CodecError {
        CodecError::Io(e)
    }
}

impl std::error::Error for CodecError {}
