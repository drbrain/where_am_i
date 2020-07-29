use super::codec::Codec;
use super::codec::CodecError;
use super::parser::Command;
use super::server::Server;

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use serde_json::Value;
use serde_json::json;

use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

use tokio_util::codec::Framed;

use tracing::debug;

pub struct Client {
    server: Arc<Mutex<Server>>,
    pub addr: SocketAddr,
    req: futures_util::stream::SplitStream<Framed<TcpStream, Codec>>,
    res: futures_util::stream::SplitSink<Framed<TcpStream, Codec>, Value>,
}

impl Client {
    pub async fn new(server: Arc<Mutex<Server>>, stream: TcpStream, addr: SocketAddr) -> io::Result<Client> {
        let (res, req) = Framed::new(stream, Codec::new()).split();

        {
            let mut s = server.lock().await;

            s.clients.insert(addr, ());
        }

        Ok(Client {
            server: server,
            addr: addr,
            req: req,
            res: res,
        })
    }

    async fn next(&mut self) -> Option<Result<Command, CodecError>> {
        self.req.next().await
    }

    async fn send(&mut self, response: Value) -> Result<(), CodecError> {
        self.res.send(response).await
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
            Some(j) => server.watch.update(j),
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

