use super::codec::Codec;
use super::parser::Command;
use super::server::Server;
use super::watch::Watch;

use crate::JsonReceiver;

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use serde_json::Value;
use serde_json::json;

use std::error::Error;
use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

use tracing::debug;
use tracing::error;
use tracing::info;

type Sender = mpsc::Sender<Value>;

pub struct Client {
    server: Arc<Mutex<Server>>,
    pub addr: SocketAddr,
    req: FramedRead<OwnedReadHalf, Codec>,
    res: Sender,
    pub watch: Arc<Mutex<Watch>>,
}

impl Client {
    pub async fn start(server: Arc<Mutex<Server>>, addr: SocketAddr, stream: TcpStream) -> io::Result<()> {
        let (read, write) = stream.into_split();
        let (res_tx, res_rx) = mpsc::channel(5);

        let client = Client::new(server, read, addr, res_tx).await?;

        start_client_rx(client).await;

        start_client_tx(write, res_rx).await;

        Ok(())
    }

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
        let original;
        let updated;

        {
            let mut watch = self.watch.lock().await;

            original = watch.clone();

            match updates {
                Some(j) => watch.update(j),
                None => (),
            };

            updated = watch.clone();
        }

        match (original.enable, updated.enable) {
            // enable
            (false, true) => self.enable_watch(updated.clone()).await,
            // disable
            (true, false) => self.disable_watch(),
            // no change
            (true, true) => (),
            (false, false) => (),
        }

        match serde_json::to_value(updated) {
            Ok(w) => w,
            Err(_) => json!({
                "class": "ERROR",
                "message": "internal error",
            }),
        }
    }

    async fn enable_watch(&self, watch: Watch) {
        debug!("enabling watch for {:?}", self.addr);
        let mut gps_rx = None;
        let mut pps_rx = None;
        let device = match watch.device {
            Some(d) => d,
            None => return,
        };

        {
            let server = self.server.lock().await;

            if watch.enable {
                gps_rx = server.gps_rx_for(device.clone());
            }

            if watch.pps {
                pps_rx = server.pps_rx_for(device.clone())
            }
        }

        match gps_rx {
            Some(rx) => relay_messages(self.res.clone(), rx),
            None => (),
        }

        match pps_rx {
            Some(rx) => relay_messages(self.res.clone(), rx),
            None => (),
        }
    }

    fn disable_watch(&self) {
        debug!("disabling watch for {:?}", self.addr);
    }
}

fn relay_messages(tx: Sender, rx: JsonReceiver) {
    tokio::spawn(async move {
        relay(tx, rx).await;
    });
}

#[tracing::instrument]
async fn relay (mut tx: Sender, mut rx: JsonReceiver) {
    loop {
        let message = rx.recv().await;

        let value = match message {
            Ok(v) => v,
            Err(e) => {
                error!("error receiving message to relay: {:?}", e);
                break;
            },
        };

        match tx.send(value).await {
            Ok(_) => (),
            Err(e) => {
                error!("error relaying message: {:?}", e);
                break;
            },
        }
    }
}

async fn start_client_rx(client: Client) {
    tokio::spawn(async move {
        client_rx(client).await;
    });
}

#[tracing::instrument]
async fn client_rx(mut client: Client) {
    match client.run().await {
        Ok(_) => info!("Client {} disconnected", client.addr),
        Err(e) => error!("Error handling client {}: {:?}", client.addr, e),
    };
}

async fn start_client_tx(write: OwnedWriteHalf, rx: mpsc::Receiver<Value>) {
    let res = FramedWrite::new(write, Codec::new());

    tokio::spawn(async move {
        client_tx(res, rx).await;
    });
}

#[tracing::instrument]
async fn client_tx(mut tx: FramedWrite<OwnedWriteHalf, Codec>, mut rx: mpsc::Receiver<Value>) {
    while let Some(value) = rx.recv().await {
        match tx.send(value).await {
            Ok(_) => (),
            Err(e) => {
                error!("Error responding to client: {:?}", e);
                break;
            },
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
         .field("peer", &self.addr)
         .finish()
    }
}

