use crate::{
    configuration::GpsdConfig,
    devices::Devices,
    gpsd::{client::Client, Response},
    pps::PPS,
};
use anyhow::Context;
use anyhow::Result;
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tracing::error;
use tracing::info;

pub struct Server {
    port: u16,
    bind_addresses: Vec<String>,
    pub clients: HashMap<SocketAddr, ()>,
    pub devices: Devices,
}

impl Server {
    pub fn new(config: GpsdConfig, devices: Devices) -> Self {
        Server {
            port: config.port,
            bind_addresses: config.bind_addresses,
            clients: HashMap::new(),
            devices,
        }
    }

    pub fn gps_rx_for(&self, device: String) -> Option<broadcast::Receiver<Response>> {
        self.devices.gps_rx_for(device)
    }

    pub fn pps_for(&self, device: String) -> Option<(PPS, watch::Receiver<i32>)> {
        self.devices.pps_rx_for(device)
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

async fn run_listener(address: &str, port: u16, server: Arc<Mutex<Server>>) -> Result<()> {
    let address = (address, port);

    let listener = TcpListener::bind(address)
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
