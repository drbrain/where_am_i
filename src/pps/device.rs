use crate::pps::PPS;
use crate::TSSender;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;

#[derive(Debug)]
pub struct Device {
    pub pps: Arc<PPS>,
    pub gps_name: String,
    pub tx: TSSender,
}

impl Device {
    pub fn new(name: String, gps_name: String) -> Result<Self> {
        let (tx, _) = broadcast::channel(5);
        let pps = Arc::new(PPS::new(name)?);

        Ok(Device { pps, gps_name, tx })
    }

    #[tracing::instrument]
    pub async fn run(&self) -> Result<()> {
        let tx = self.tx.clone();
        let pps = self.pps.clone();

        tokio::spawn(async move {
            send_pps_events(pps, tx).await;
        });

        Ok(())
    }
}

async fn send_pps_events(arc_pps: Arc<PPS>, tx: TSSender) {
    let pps = Arc::try_unwrap(arc_pps).unwrap();
    tokio::pin!(pps);

    while let Some(timestamp) = pps.next().await {
        if let Err(_) = tx.send(timestamp) {
            break;
        }
    }
}
