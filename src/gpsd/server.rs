use crate::configuration::GpsConfig;
use crate::configuration::GpsdConfig;
use crate::gps::GPS;
use crate::gpsd::client::Client;
use crate::gpsd::Response;
use crate::nmea;
use crate::pps;
use crate::shm::NtpShm;
use crate::TSReceiver;
use crate::TSSender;
use anyhow::Context;
use anyhow::Result;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio_serial::SerialPortBuilder;
use tracing::error;
use tracing::info;

pub struct Server {
    port: u16,
    bind_addresses: Vec<String>,
    pub clients: HashMap<SocketAddr, ()>,
    pub devices: Vec<GpsConfig>,
    gps_tx: HashMap<String, broadcast::Sender<Response>>,
    pps_tx: HashMap<String, TSSender>,
}

impl Server {
    pub fn new(config: GpsdConfig, devices: Vec<GpsConfig>) -> Self {
        Server {
            port: config.port,
            bind_addresses: config.bind_addresses,
            clients: HashMap::new(),
            devices,
            gps_tx: HashMap::new(),
            pps_tx: HashMap::new(),
        }
    }

    pub fn add_gps(&mut self, gps: &GPS) {
        self.gps_tx.insert(gps.name.clone(), gps.gpsd_tx.clone());
    }

    pub fn add_pps(&mut self, pps: &pps::Device, name: String) {
        self.pps_tx.insert(name, pps.tx.clone());
    }

    pub fn gps_rx_for(&self, device: String) -> Option<broadcast::Receiver<Response>> {
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

    pub async fn start_gps_devices(&mut self) {
        for gps_config in self.devices.clone().iter() {
            self.start_gps_device(gps_config).await;
        }
    }

    async fn start_gps_device(&mut self, gps_config: &GpsConfig) {
        let name = gps_config.name.clone();
        let gps_name = gps_config.device.clone();
        let messages = gps_config.messages.clone().unwrap_or_default();

        let serial_port_settings = match SerialPortBuilder::try_from(gps_config.clone()) {
            Ok(s) => s,
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            }
        };

        let device = nmea::Device::new(
            gps_name.clone(),
            gps_config.gps_type.clone(),
            serial_port_settings,
            messages,
        );

        let gps_tx = device.run().await;

        let mut gps = GPS::new(gps_name.clone(), gps_tx.clone());

        gps.read().await;

        self.add_gps(&gps);

        info!("registered GPS {}", name.clone());

        if let Some(ntp_unit) = gps_config.ntp_unit {
            NtpShm::new(-1)
                .relay(ntp_unit, gps.ntp_tx.subscribe())
                .await;
            info!("Sending GPS time from {} via NTP unit {}", name, ntp_unit);
        }

        match &gps_config.pps {
            Some(pps_config) => {
                let pps_name = pps_config.device.clone();

                let mut pps = pps::Device::new(pps_name.clone(), gps_name.clone());

                match pps.run().await {
                    Ok(()) => (),
                    Err(e) => {
                        error!("Error opening PPS device {}: {}", pps_name, e);
                        std::process::exit(1);
                    }
                };

                self.add_pps(&pps, gps_name.clone());

                info!("registered PPS {} under {}", pps_name, gps_name);

                if let Some(ntp_unit) = pps_config.ntp_unit {
                    NtpShm::new(-20).relay(ntp_unit, pps.tx.subscribe()).await;
                    info!(
                        "Sending PPS time from {} via NTP unit {}",
                        pps_name, ntp_unit
                    );
                }

                Some(pps)
            }
            None => None,
        };
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
