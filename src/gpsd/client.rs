use super::codec::Codec;
use super::codec::CodecError;
use super::parser::Command;
use super::parser::json_to_string;
use super::server::Server;

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

pub struct Client {
    server: Arc<Mutex<Server>>,
    pub addr: SocketAddr,
    framed: Framed<TcpStream, Codec>,
}

impl Client {
    pub async fn new(server: Arc<Mutex<Server>>, stream: TcpStream, addr: SocketAddr) -> io::Result<Client> {
        let framed = Framed::new(stream, Codec::new());

        {
            let mut s = server.lock().await;

            s.clients.insert(addr, ());
        }

        Ok(Client {
            server: server,
            addr: addr,
            framed: framed
        })
    }

    async fn next(&mut self) -> Option<Result<Command, CodecError>> {
        self.framed.next().await
    }

    async fn send(&mut self, response: Value) -> Result<(), CodecError> {
        self.framed.send(response).await
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        while let Some(result) = self.next().await {
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
                Command::Watch(w) => self.command_watch(w).await,
            };

            debug!("{:?}", response);
            self.send(response).await?;
        }

        {
            let mut server = self.server.lock().await;
            server.clients.remove(&self.addr);
        }

        Ok(())
    }

    async fn command_watch(&self, updates: Option<Value>) -> Value {
        let mut server = self.server.lock().await;

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
                    server.watch.device = json_to_string(&j["device"]);
                }

                if j["remote"].is_string() {
                    server.watch.remote = json_to_string(&j["remote"]);
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
}

