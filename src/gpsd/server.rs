use super::codec::Codec;
use super::client::Client;
use super::super::gps::GPS;
use super::super::pps::PPS;

use crate::JsonSender;

use futures_util::sink::SinkExt;

use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use tokio_util::codec::FramedWrite;

use tracing::error;
use tracing::info;

#[derive(Debug)]
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

            let (read, write) = stream.into_split();
            let (res_tx, mut res_rx) = mpsc::channel(5);

            let mut client = Client::new(server, read, addr, res_tx).await?;

            tokio::spawn(async move {
                match client.run().await {
                    Ok(_) => info!("Client {} disconnected", client.addr),
                    Err(e) => error!("Error handling client {}: {:?}", client.addr, e),
                };
            });

            let mut res = FramedWrite::new(write, Codec::new());

            tokio::spawn(async move {
                while let Some(value) = res_rx.recv().await {
                    match res.send(value).await {
                        Ok(_) => (),
                        Err(e) => error!("Error responding to client: {:?}", e),
                    }
                }
            });
        }
    }
}

