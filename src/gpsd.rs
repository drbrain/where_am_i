mod parser;
mod codec;
mod client;

use super::gps::GPS;
use super::pps::PPS;

use parser::Command;
use codec::Codec;
use codec::CodecError;

use crate::JsonSender;
use crate::gpsd::client::client;

use serde::Deserialize;
use serde::Serialize;

use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;

use tracing::debug;
use tracing::error;
use tracing::info;

#[derive(Debug)]
pub struct Server {
    port: u16,
    clients: HashMap<SocketAddr, ()>,
    gps_tx: HashMap<String, JsonSender>,
    pps_tx: HashMap<String, JsonSender>,
    watch: Watch,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Server {
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

