mod parser;
mod codec;

use super::gps::GPS;
use super::pps::PPS;

use parser::Command;
use codec::Codec;
use codec::CodecError;

use crate::JsonSender;
use crate::JsonReceiver;

use futures::SinkExt;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::Mutex;
use tokio::sync::broadcast;

use tokio_util::codec::Framed;

use tracing::debug;
use tracing::error;
use tracing::info;

#[derive(Debug)]
pub struct GpsdServer {
    port: u16,
    clients: HashMap<SocketAddr, ()>,
    gps_tx: HashMap<String, JsonSender>,
    pps_tx: HashMap<String, JsonSender>,
    watch: Watch,
}

impl GpsdServer {
    pub fn new(port: u16) -> Self {
        GpsdServer {
            port: port,
            clients: HashMap::new(),
            gps_tx: HashMap::new(),
            pps_tx: HashMap::new(),
            watch: Watch { class: "WATCH".to_string(), ..Default::default() },
        }
    }

    pub fn add_gps(&mut self, gps: GPS) {
        self.gps_tx.insert(gps.name.clone(), gps.tx.clone());
    }

    pub fn add_pps(&mut self, pps: PPS) {
        self.pps_tx.insert(pps.name.clone(), pps.tx.clone());
    }

    #[tracing::instrument]
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let port = self.port;

        let server = Arc::new(Mutex::new(self));
        let address = ("0.0.0.0", port);

        let mut listener = TcpListener::bind(address).await?;
        info!("listening on {} port {}", listener.local_addr()?.ip(), port);

        loop {
            let (stream, addr) = listener.accept().await?;

            let server = Arc::clone(&server);

            tokio::spawn(async move {
                match client(server, stream, addr).await {
                    Ok(_) => debug!("client {:?} disconnected", addr),
                    Err(e) => error!("client {:?} errored: {:?}", addr, e),
                }
            });
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct Watch {
    class: String,
    enable: bool,
    json: bool,
    nmea: bool,
    raw: u64,
    scaled: bool,
    split24: bool,
    pps: bool,
    device: Option<String>,
    remote: Option<String>,
}

type ValueReceiver = broadcast::Receiver<Value>;

struct Client {
    client: Framed<TcpStream, Codec>,
}

impl Client {
    async fn new(server: Arc<Mutex<GpsdServer>>, client: Framed<TcpStream, Codec>) -> io::Result<Client> {
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

async fn client(server: Arc<Mutex<GpsdServer>>, stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
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

async fn command_watch(server: Arc<Mutex<GpsdServer>>, updates: Option<Value>) -> Value {
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
