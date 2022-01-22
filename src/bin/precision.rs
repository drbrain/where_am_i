use anyhow::Result;
use clap::Parser;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
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

    let pps = OpenOptions::new()
        .read(true)
        .write(true)
        .open(device.clone())?;

    let pps = PPS::new(device.clone(), pps.as_raw_fd());
    info!("Opened PPS device {}", device.clone());

    let precision = Precision::default();
    info!(
        "PPS {} precision: {}",
        device.clone(),
        precision.measure_precision(pps).await?
    );

    Ok(())
}
