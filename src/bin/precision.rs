use anyhow::Result;
use clap::Parser;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();

    let device = args.pps_device.clone();

    let pps = PPS::new(device.clone())?;
    info!("Opened PPS device {}", device.clone());

    info!(
        "PPS {} precision: {}",
        device.clone(),
        Precision::new().measure(pps).await?
    );

    Ok(())
}
