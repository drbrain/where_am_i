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
use std::time::Duration;
use std::time::SystemTime;

use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

use tracing::debug;

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Codec {
    last_received: Option<Duration>,
}

type VE<'a> = VerboseError<&'a [u8]>;

impl Decoder for Codec {
    type Item = NMEA;
    type Error = CodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let last_received = match self.last_received {
            Some(r) => r,
            None => timestamp(),
        };

        let bytes = buf.to_bytes();
        let input = bytes.bytes();

        match parse::<VE>(input, last_received) {
            Ok((input, nmea)) => {
                buf.extend_from_slice(&Bytes::copy_from_slice(input));

                self.last_received = None;

                Ok(Some(nmea))
            }
            Err(Err::Incomplete(_)) => {
                buf.extend_from_slice(&Bytes::copy_from_slice(input));

                self.last_received = Some(last_received);

                Ok(None)
            }
            Err(Err::Error(_)) => panic!("impossible error!"),
            Err(Err::Failure(_)) => panic!("impossible failure!"),
        }
    }
}

fn timestamp() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
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
