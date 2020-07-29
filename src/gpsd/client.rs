use super::Codec;
use super::CodecError;
use super::Command;
use super::Server;
use super::parser;

use futures::SinkExt;

use serde_json::Value;
use serde_json::json;

use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::Mutex;

use tokio_util::codec::Framed;

use tracing::debug;

struct Client {
    client: Framed<TcpStream, Codec>,
}

impl Client {
    async fn new(server: Arc<Mutex<Server>>, client: Framed<TcpStream, Codec>) -> io::Result<Client> {
        let addr = client.get_ref().peer_addr()?;

        let mut server = server.lock().await;

        server.clients.insert(addr, ());

        Ok(Client { client })
    }

    async fn next(&mut self) -> Option<Result<Command, CodecError>> {
        self.client.next().await
    }

    async fn send(&mut self, response: Value) -> Result<(), CodecError> {
        self.client.send(response).await
    }
}

pub async fn client(server: Arc<Mutex<Server>>, stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let framed = Framed::new(stream, Codec::new());
    let mut client = Client::new(server.clone(), framed).await?;

    while let Some(result) = client.next().await {
        let command = match result {
            Ok(c) => c,
            Err(_) => Command::Error("unrecognized command".to_string()),
        };

        let response = match command {
            Command::Devices => json!({
                "class": "DEVICES",
                "devices": [],
            }),
            Command::Device(_) => json!({
                "class": "DEVICE",
                "stopbits": 1,
            }),
            Command::Error(e) => json!({
                "class": "ERROR",
                "message": e,
            }),
            Command::Poll => json!({
                "class": "POLL",
                "time": 0,
                "active": 0,
                "tpv": [],
                "sky": [],
            }),
            Command::Version => json!({
                "class": "VERSION",
                "release": "",
                "rev": "",
                "proto_major": 3,
                "proto_minor": 10,
            }),
            Command::Watch(w) => command_watch(server.clone(), w).await,
        };

        debug!("{:?}", response);
        client.send(response).await?;
    }

    {
        let mut server = server.lock().await;
        server.clients.remove(&addr);
    }

    Ok(())
}

async fn command_watch(server: Arc<Mutex<Server>>, updates: Option<Value>) -> Value {
    let mut server = server.lock().await;

    match updates {
        Some(j) => {
            if j["enable"].is_boolean() {
                server.watch.enable = j["enable"].as_bool().unwrap_or(false);
            }

            if j["json"].is_boolean() {
                server.watch.json = j["json"].as_bool().unwrap_or(false);
            }

            if j["nmea"].is_boolean() {
                server.watch.nmea = j["nmea"].as_bool().unwrap_or(false);
            }

            if j["raw"].is_u64() {
                server.watch.raw = j["raw"].as_u64().unwrap_or(0);
            }

            if j["scaled"].is_boolean() {
                server.watch.scaled = j["scaled"].as_bool().unwrap_or(false);
            }

            if j["split24"].is_boolean() {
                server.watch.split24 = j["split24"].as_bool().unwrap_or(false);
            }

            if j["pps"].is_boolean() {
                server.watch.pps = j["pps"].as_bool().unwrap_or(false);
            }

            if j["device"].is_string() {
                server.watch.device = parser::json_to_string(&j["device"]);
            }

            if j["remote"].is_string() {
                server.watch.remote = parser::json_to_string(&j["remote"]);
            }
        },
        None => (),
    };

    match serde_json::to_value(&server.watch) {
        Ok(w) => w,
        Err(_) => json!({
            "class": "ERROR",
            "message": "internal error",
        }),
    }
}

