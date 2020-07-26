use super::command_parser;
use super::command_parser::Command;

use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;

use serde_json;
use serde_json::Value;
use serde::Serialize;

use std::cmp;
use std::fmt;
use std::io;
use std::str;

use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GpsdCodec {
    next_index: usize,
    max_length: usize,
    is_discarding: bool,
}

impl GpsdCodec {
    pub fn new() -> GpsdCodec {
        GpsdCodec {
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

impl Decoder for GpsdCodec {
    type Item = Command;
    type Error = GpsdCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Command>, GpsdCodecError> {
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
                        return Err(GpsdCodecError::UnrecognizedRequest);
                    }
                }
                (false, Some(offset)) => {
                    // Found a line!
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let line = buf.split_to(newline_index + 1);
                    let line = &line[..line.len()];
                    let line = utf8(line)?;
                    return Ok(Some(command_parser::parse(line)));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(GpsdCodecError::UnrecognizedRequest);
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

impl<T> Encoder<T> for GpsdCodec
where
    T: Serialize,
{
    type Error = GpsdCodecError;

    fn encode(&mut self, json: T, buf: &mut BytesMut) -> Result<(), GpsdCodecError> {
        let out = serde_json::to_string(&json).unwrap();

        buf.reserve(out.len() + 1);
        buf.put(out.as_bytes());
        buf.put_u8(b'\n');

        Ok(())
    }
}

impl Default for GpsdCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum GpsdCodecError {
    UnrecognizedRequest,
    Io(io::Error),
}

impl fmt::Display for GpsdCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpsdCodecError::UnrecognizedRequest => write!(f, "unrecognized request"),
            GpsdCodecError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<io::Error> for GpsdCodecError {
    fn from(e: io::Error) -> GpsdCodecError {
        GpsdCodecError::Io(e)
    }
}

impl std::error::Error for GpsdCodecError {}
