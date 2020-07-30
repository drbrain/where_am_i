use super::codec::Codec;
use super::parser::Command;
use super::server::Server;
use super::watch::Watch;

use futures_util::stream::StreamExt;

use serde_json::Value;
use serde_json::json;

use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::tcp::OwnedReadHalf;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use tokio_util::codec::FramedRead;

use tracing::debug;

type Sender = mpsc::Sender<Value>;

pub struct Client {
    server: Arc<Mutex<Server>>,
    pub addr: SocketAddr,
    req: FramedRead<OwnedReadHalf, Codec>,
    res: Sender,
    pub watch: Arc<Mutex<Watch>>,
}

impl Client {
    pub async fn new(server: Arc<Mutex<Server>>, read: OwnedReadHalf, addr: SocketAddr, res: Sender) -> io::Result<Client> {
        let req = FramedRead::new(read, Codec::new());

        {
            let mut s = server.lock().await;

            s.clients.insert(addr, ());
        }

        let watch = Watch { class: "WATCH".to_string(), ..Default::default() };

        Ok(Client {
            server: server,
            addr: addr,
            req: req,
            res: res,
            watch: Arc::new(Mutex::new(watch)),
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        while let Some(result) = self.req.next().await {
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
            self.res.send(response).await?;
        }

        {
            let mut server = self.server.lock().await;
            server.clients.remove(&self.addr);
        }

        Ok(())
    }

    async fn command_watch(&self, updates: Option<Value>) -> Value {
        let mut watch = self.watch.lock().await;

        match updates {
            Some(j) => watch.update(j),
            None => (),
        };

        let updated = watch.clone();

        match serde_json::to_value(updated) {
            Ok(w) => w,
            Err(_) => json!({
                "class": "ERROR",
                "message": "internal error",
            }),
        }
    }
}

