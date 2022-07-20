use anyhow::{Context, Result};
use prometheus_hyper::Server;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Notify;
use tracing::info;

pub struct Exporter {
    bind_address: SocketAddr,
    shutdown: Arc<Notify>,
}

impl Exporter {
    pub fn new(bind_address: String) -> Result<Self> {
        let bind_address: SocketAddr = bind_address
            .parse()
            .with_context(|| format!("Can't parse prometheus listen address {}", bind_address))?;

        let shutdown = Arc::new(Notify::new());

        let exporter = Exporter {
            bind_address,
            shutdown,
        };

        Ok(exporter)
    }

    async fn run(&self) {
        info!("Starting prometheus server on {}", self.bind_address);

        let _ = Server::run(
            Arc::new(prometheus::default_registry().clone()),
            self.bind_address,
            self.shutdown.notified(),
        )
        .await;
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }
}
