use crate::pps::ioctl;
use crate::pps::FetchFuture;
use crate::pps::Error;
use crate::JsonSender;

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

use tokio::fs::File;
use tokio::sync::broadcast;

use tracing::error;
use tracing::info;

#[derive(Debug)]
pub struct PPS {
    pub name: String,
    pub gps_name: String,
    pub tx: JsonSender,
}

impl PPS {
    pub fn new(name: String, gps_name: String) -> Self {
        let (tx, _) = broadcast::channel(5);

        PPS { name, gps_name, tx }
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

        let name = self.name.clone();
        let tx = self.tx.clone();
        let gps_name = self.gps_name.clone();

        tokio::spawn(async move {
            let fd = pps.as_raw_fd();

            info!("watching PPS events on {}", name);

            loop {
                let mut pps_data = match FetchFuture::new(fd).await {
                    Ok(d) => d,
                    Err(e) => {
                        error!("fetch error on {} ({:?})", name, e);
                        continue;
                    }
                };

                pps_data["device"] = gps_name.clone().into();

                if let Err(_e) = tx.send(pps_data) {
                    // error!("send error: {:?}", e);
                }
            }
        });

        Ok(())
    }
}

