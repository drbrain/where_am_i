use anyhow::Context;
use anyhow::Result;

use crate::configuration::GpsdConfig;
use crate::gps::GPS;
use crate::gpsd::client::Client;
use crate::pps::Device;
use crate::JsonReceiver;
use crate::JsonSender;
use crate::TSReceiver;
use crate::TSSender;

use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;

use tracing::error;
use tracing::info;

pub struct Server {
    port: u16,
    bind_addresses: Vec<String>,
    pub clients: HashMap<SocketAddr, ()>,
    gps_tx: HashMap<String, JsonSender>,
    pps_tx: HashMap<String, TSSender>,
}

impl Server {
    pub fn new(config: &GpsdConfig) -> Self {
        Server {
            port: config.port,
            bind_addresses: config.bind_addresses.clone(),
            clients: HashMap::new(),
            gps_tx: HashMap::new(),
            pps_tx: HashMap::new(),
        }
    }

    pub fn add_gps(&mut self, gps: &GPS) {
        self.gps_tx.insert(gps.name.clone(), gps.gpsd_tx.clone());
    }

    pub fn add_pps(&mut self, pps: &Device, name: String) {
        self.pps_tx.insert(name, pps.tx.clone());
    }

    pub fn gps_rx_for(&self, device: String) -> Option<JsonReceiver> {
        if let Some(tx) = self.gps_tx.get(&device) {
            return Some(tx.subscribe());
        }

        None
    }

    #[tracing::instrument]
    pub fn pps_rx_for(&self, device: String) -> Option<TSReceiver> {
        if let Some(tx) = self.pps_tx.get(&device) {
            return Some(tx.subscribe());
        }

        None
    }

    pub async fn run(self) -> Result<()> {
        let port = self.port;
        let addresses = self.bind_addresses.clone();
        let server = Arc::new(Mutex::new(self));

        for address in &addresses {
            run_listener(address, port, Arc::clone(&server)).await?;
        }

        Ok(())
    }
}

#[tracing::instrument]
async fn run_listener(address: &String, port: u16, server: Arc<Mutex<Server>>) -> Result<()> {
    let address = (address.as_str(), port);

    let mut listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("Failed to bind to {}:{}", address.0, address.1))?;

    let listen_address = listener.local_addr().with_context(|| {
        format!(
            "Unable to determine listen address after binding {}:{}",
            address.0, address.1
        )
    })?;
    info!("listening on {} port {}", listen_address.ip(), port);

    loop {
        let (stream, addr) = listener.accept().await?;

        let server = Arc::clone(&server);

        match Client::start(server, addr, stream).await {
            Ok(()) => (),
            Err(e) => error!("failed to start client: {:?}", e),
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
