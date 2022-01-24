use anyhow::Result;
use clap::Parser;
use std::ops::Deref;
use tracing::info;
use where_am_i::pps::PPS;
use where_am_i::precision::Precision;

/// Show PPS precision
#[derive(Parser)]
#[clap(about)]
struct Args {
    /// PPS device path
    #[clap(long, default_value = "/dev/pps0")]
    pub pps_device: String,
    /// Continue to show precision measurements after the first result
    #[clap(long)]
    pub watch: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();

    let device = args.pps_device.clone();

    let pps = PPS::new(device.clone())?;
    info!("Opened PPS device {}", device.clone());

    let precision = Precision::new();

    if args.watch {
        let mut precision = precision.watch(pps).await;

        while let Ok(_) = precision.changed().await {
            let current = *precision.borrow().deref();

            info!("PPS {} precision: {}", &device, current);
        }
    } else {
        info!("PPS {} precision: {}", &device, precision.once(pps).await?);
    }

    Ok(())
}
