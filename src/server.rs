mod command_parser;

use crate::JsonSender;
use crate::JsonReceiver;

use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;

use tracing::info;

#[derive(Clone)]
struct Client {
    json: bool,
    pps: bool,
    watch: bool,
}

impl Client {
    fn new() -> Client {
        Client {
            json: false,
            pps: false,
            watch: false,
        }
    }
}

#[derive(Clone)]
struct Clients(Arc<RefCell<HashMap<SocketAddr, Client>>>);

impl Clients {
    fn new() -> Clients {
        Clients(Arc::new(RefCell::new(HashMap::new())))
    }

    fn add(&self, addr: SocketAddr, client: Client) {
        self.0.borrow_mut().insert(addr, client);
    }

    fn remove(&self, addr: &SocketAddr) -> Option<Client> {
        self.0.borrow_mut().remove(addr)
    }
}

#[tracing::instrument]
pub async fn spawn(port: u16, tx: JsonSender) {
    let address = ("0.0.0.0", port);

    let mut listener = TcpListener::bind(address).await.unwrap();
    let clienst = Clients::new();

    loop {
        info!("waiting for client");

        let (socket, addr) = listener.accept().await.unwrap();

        info!("client connected {:?}", addr);

        let gpsd = Framed::new(socket, GpsdCodec::new());
    }
}

use bytes::Buf;
use bytes::BytesMut;

use std::fmt;
use std::io;
use std::str;

use tokio_util::codec::Framed;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

pub enum Command {
    Device,
    Devices,
    Poll,
    Version,
    Watch,
}

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

fn without_carriage_return(s: &[u8]) -> &[u8] {
    if let Some(&b'\r') = s.last() {
        &s[..s.len() - 1]
    } else {
        s
    }
}

fn decode_line(line: String) -> Result<Option<Command>, GpsdCodecError> {
    todo!()
}

impl Decoder for GpsdCodec {
    type Item = String;
    type Error = GpsdCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<String>, GpsdCodecError> {
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
                    let line = &line[..line.len() - 1];
                    let line = without_carriage_return(line);
                    let line = utf8(line)?;
                    return Ok(Some(line.to_string()));
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

