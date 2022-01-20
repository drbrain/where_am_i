use super::parser;
use super::parser::Command;
use crate::gpsd::ErrorMessage;
use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;
use serde::Serialize;
use std::cmp;
use std::fmt;
use std::io;
use std::str;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

use tracing::trace;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Codec {
    next_index: usize,
    max_length: usize,
    is_discarding: bool,
}

impl Codec {
    pub fn new() -> Codec {
        Codec {
            next_index: 0,
            max_length: 80,
            is_discarding: false,
        }
    }
}

fn utf8(buf: &[u8]) -> Result<&str, io::Error> {
    str::from_utf8(buf)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unable to decode input as UTF8"))
}

impl Decoder for Codec {
    type Item = Command;
    type Error = CodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Command>, CodecError> {
        loop {
            let read_to = cmp::min(81, buf.len());

            let newline_offset = buf[self.next_index..read_to]
                .iter()
                .position(|b| *b == b'\n');

            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    buf.advance(offset + self.next_index + 1);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Err(CodecError::UnrecognizedRequest);
                    }
                }
                (false, Some(offset)) => {
                    // Found a line!
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let line = buf.split_to(newline_index + 1);
                    let line = &line[..line.len()];
                    let line = utf8(line)?;
                    let command = parser::parse(line);
                    trace!("GPSD received command {:?}", command);
                    return Ok(Some(command));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(CodecError::UnrecognizedRequest);
                }
                (false, None) => {
                    // We didn't find a line or reach the length limit, so the next
                    // call will resume searching at the current offset.
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }
}

impl<T> Encoder<T> for Codec
where
    T: Serialize,
{
    type Error = CodecError;

    fn encode(&mut self, json: T, buf: &mut BytesMut) -> Result<(), CodecError> {
        let serialized = serde_json::to_string(&json);

        let out = match serialized {
            Ok(s) => s,
            Err(_) => {
                let internal_error = ErrorMessage {
                    message: "internal error".to_string(),
                };

                match serde_json::to_string(&internal_error) {
                    Ok(s) => s,
                    Err(_) => return Err(CodecError::InternalError),
                }
            }
        };

        buf.reserve(out.len() + 1);
        buf.put(out.as_bytes());
        buf.put_u8(b'\n');

        trace!("GPSD sent {:?}", out);

        Ok(())
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::new()
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
