use crate::pps::ioctl;
use crate::pps::Error;
use crate::pps::PPS;
use crate::TSSender;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use tokio::fs::File;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tracing::info;

#[derive(Debug)]
pub struct Device {
    pub name: String,
    pub gps_name: String,
    pub tx: TSSender,
}

impl Device {
    pub fn new(name: String, gps_name: String) -> Self {
        let (tx, _) = broadcast::channel(5);

        Device { name, gps_name, tx }
    }

    #[tracing::instrument]
    fn open(&mut self) -> Result<File, Box<dyn std::error::Error>> {
        let pps = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.name.clone())?;

        info!("Opened {}", self.name);

        Ok(File::from_std(pps))
    }

    #[tracing::instrument]
    fn configure(&self, pps: File) -> Result<File, Box<dyn std::error::Error>> {
        let pps_fd = pps.as_raw_fd();

        unsafe {
            let mut mode = 0;

            if ioctl::getcap(pps_fd, &mut mode).is_err() {
                return Err(Box::new(Error::CapabilitiesFailed(self.name.clone())));
            };

            if mode & ioctl::CANWAIT == 0 {
                return Err(Box::new(Error::CannotWait(self.name.clone())));
            };

            if (mode & ioctl::CAPTUREASSERT) == 0 {
                return Err(Box::new(Error::CannotCaptureAssert(self.name.clone())));
            };

            let mut params = ioctl::params::default();

            if ioctl::getparams(pps_fd, &mut params).is_err() {
                return Err(Box::new(Error::CannotGetParameters(self.name.clone())));
            };

            params.mode |= ioctl::CAPTUREASSERT;

            if ioctl::setparams(pps_fd, &mut params).is_err() {
                return Err(Box::new(Error::CannotSetParameters(self.name.clone())));
            };
        }

        Ok(pps)
    }

    #[tracing::instrument]
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let file = self.open()?;
        let pps = self.configure(file)?;

        let pps_name = self.name.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            send_pps_events(pps, tx, pps_name).await;
        });

        Ok(())
    }
}

async fn send_pps_events(pps: File, tx: TSSender, pps_name: String) {
    let fd = pps.as_raw_fd();

    info!("watching PPS events on {}", pps_name);
    let pps = PPS::new(pps_name.clone(), -20, fd);

    tokio::pin!(pps);

    while let Some(timestamp) = pps.next().await {
        if let Err(_e) = tx.send(timestamp) {
            // error!("send error: {:?}", e);
        }
    }
}
