use crate::JsonReceiver;
use crate::JsonSender;
use crate::gps::GPS;
use crate::gpsd::client::Client;
use crate::pps::PPS;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;

use tracing::error;
use tracing::info;

pub struct Server {
    port: u16,
    pub clients: HashMap<SocketAddr, ()>,
    gps_tx: HashMap<String, JsonSender>,
    pps_tx: HashMap<String, JsonSender>,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Server {
            port: port,
            clients: HashMap::new(),
            gps_tx: HashMap::new(),
            pps_tx: HashMap::new(),
        }
    }

    pub fn add_gps(&mut self, gps: GPS) {
        self.gps_tx.insert(gps.name.clone(), gps.tx.clone());
    }

    pub fn add_pps(&mut self, pps: PPS) {
        self.pps_tx.insert(pps.name.clone(), pps.tx.clone());
    }

    pub fn gps_rx_for(&self, device: String) -> Option<JsonReceiver> {
        if let Some(tx) = self.gps_tx.get(&device) {
            return Some(tx.subscribe());
        }

        None
    }

    pub fn pps_rx_for(&self, device: String) -> Option<JsonReceiver> {
        if let Some(tx) = self.pps_tx.get(&device) {
            return Some(tx.subscribe());
        }

        None
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

            match Client::start(server, addr, stream).await {
                Ok(()) => (),
                Err(e) => error!("failed to start client: {:?}", e),
            }
        }
    }

}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("port", &self.port)
            .field("clients", &self.clients.len())
            .finish()
    }
}

