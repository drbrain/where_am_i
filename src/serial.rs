use tokio::io::BufReader;

use tokio_serial::Serial;
use tokio_serial::SerialPortSettings;

pub async fn open(device: String, settings: SerialPortSettings) -> BufReader<Serial> {
    let sp = match Serial::from_path(&device, &settings) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error {}", e);
            std::process::exit(1);
        }
    };

    let gps = BufReader::new(sp);

    return gps;
}

