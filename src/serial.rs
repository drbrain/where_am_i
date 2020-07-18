use tokio::io::BufReader;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

use tracing::error;
use tracing::info;

#[tracing::instrument]
pub async fn open(device: String, settings: SerialPortSettings) -> BufReader<Serial> {
    let sp = match Serial::from_path(&device, &settings) {
        Ok(s) => s,
        Err(e) => {
            error!("Error opening GPS {} ({})", device, e);
            std::process::exit(1);
        }
    };

    info!("Opened GPS device {}", device);

    let gps = BufReader::new(sp);

    return gps;
}

