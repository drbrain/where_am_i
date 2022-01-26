use crate::configuration::GpsConfig;
use crate::configuration::GpsdConfig;
use crate::gps::GPS;
use crate::gpsd::client::Client;
use crate::gpsd::Response;
use crate::pps::PPS;
use crate::precision::Precision;
use crate::shm::NtpShm;
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
    pub devices: Vec<GpsConfig>,
    gps_tx: HashMap<String, broadcast::Sender<Response>>,
    pps: HashMap<String, (PPS, watch::Receiver<i32>)>,
}

impl Server {
    pub fn new(config: GpsdConfig, devices: Vec<GpsConfig>) -> Self {
        Server {
            port: config.port,
            bind_addresses: config.bind_addresses,
            clients: HashMap::new(),
            devices,
            gps_tx: HashMap::new(),
            pps: HashMap::new(),
        }
    }

    pub fn add_gps(&mut self, gps: &GPS) {
        self.gps_tx.insert(gps.name.clone(), gps.gpsd_tx.clone());
    }

    pub async fn add_pps(
        &mut self,
        pps: &PPS,
        current_precision: watch::Receiver<i32>,
        name: String,
    ) {
        self.pps.insert(name, (pps.clone(), current_precision));
    }

    pub fn gps_rx_for(&self, device: String) -> Option<broadcast::Receiver<Response>> {
        if let Some(tx) = self.gps_tx.get(&device) {
            return Some(tx.subscribe());
        }

        None
    }

    #[tracing::instrument]
    pub fn pps_for(&self, device: String) -> Option<(PPS, watch::Receiver<i32>)> {
        match self.pps.get(&device) {
            Some((pps, precision)) => Some((pps.clone(), precision.clone())),
            None => None,
        }
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

    pub async fn start_gps_devices(&mut self) -> Result<()> {
        for gps_config in self.devices.clone().iter() {
            self.start_gps_device(gps_config).await?;
        }

        Ok(())
    }

    async fn start_gps_device(&mut self, gps_config: &GpsConfig) -> Result<()> {
        let name = gps_config.name.clone();
        let gps_name = gps_config.device.clone();

        let mut gps = GPS::new(gps_config).await?;

        gps.read().await;

        self.add_gps(&gps);

        info!("registered GPS {}", name.clone());

        if let Some(ntp_unit) = gps_config.ntp_unit {
            let ntp_shm = NtpShm::new(ntp_unit);

            ntp_shm.relay(0, -1, gps.ntp_tx.subscribe()).await;
            info!("Sending GPS time from {} via NTP unit {}", name, ntp_unit);
        }

        match &gps_config.pps {
            Some(pps_config) => {
                let pps_name = pps_config.device.clone();

                let pps = PPS::new(pps_name.clone()).unwrap();
                let current_precision = Precision::new().watch(pps.clone()).await;

                self.add_pps(&pps, current_precision.clone(), gps_name.clone())
                    .await;

                info!("registered PPS {} under {}", pps_name, gps_name);

                if let Some(ntp_unit) = pps_config.ntp_unit {
                    let ntp_shm = NtpShm::new(ntp_unit);
                    ntp_shm
                        .relay_pps(current_precision, 0, pps.current_timestamp())
                        .await;
                    info!(
                        "Sending PPS time from {} via NTP unit {}",
                        pps_name, ntp_unit
                    );
                }

                Some(pps)
            }
            None => None,
        };

        Ok(())
    }
}

#[tracing::instrument]
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
