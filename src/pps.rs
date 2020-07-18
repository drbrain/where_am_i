mod ioctl;

use crate::JsonSender;

use json::object;

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::time::SystemTime;

use tokio::fs::File;

use tracing::debug;
use tracing::error;
use tracing::info;

#[tracing::instrument]
pub fn spawn(device: String, tx: JsonSender) {
    let pps = match OpenOptions::new().read(true).write(true).open(&device) {
        Ok(p) => p,
        Err(e) => {
            error!("Error opening PPS {} ({})", device, e);
            std::process::exit(1);
        }
    };

    info!("Opened {}", device);
    let pps = File::from_std(pps);

    match configure(pps.as_raw_fd()) {
        Ok(_) => (),
        Err(e) => {
            error!("configuring PPS device ({:?})", e);
            std::process::exit(1);
        }
    };

    tokio::spawn(async move {
        let mut data = ioctl::data::default();
        let data_ptr: *mut ioctl::data = &mut data;
        info!("watching PPS events on {}", device);

        loop {
            data.timeout.flags = ioctl::TIME_INVALID;

            unsafe {
                match ioctl::fetch(pps.as_raw_fd(), data_ptr) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("fetch error on {} ({:?})", device, e);
                        continue;
                    }
                }
            };

            let received = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n,
                Err(_) => {
                    error!("error getting current time");
                    continue;
                },
            };

            let pps_obj = object! {
                class:      "PPS".to_string(),
                device:     "".to_string(),
                real_sec:   data.info.assert_tu.sec,
                real_nsec:  data.info.assert_tu.nsec,
                clock_sec:  received.as_secs(),
                clock_nsec: received.subsec_nanos(),
                precision:  -1,
            };

            match tx.send(pps_obj) {
                Ok(_)  => debug!("sent tick"),
                Err(e) => error!("send error: {:?}", e),
            }
        }
    });
}

#[tracing::instrument]
fn configure(pps_fd: i32) -> Result<bool, String> {
    unsafe {
        let mut mode = 0;

        match ioctl::getcap(pps_fd, &mut mode) {
            Ok(_) => (),
            Err(_) => return Err("unable to get capabilities".to_string()),
        }

        if mode & ioctl::CANWAIT == 0 {
            return Err("cannot wait".to_string());
        }

        if (mode & ioctl::CAPTUREASSERT) == 0 {
            return Err("cannot capture asserts".to_string());
        }

        let mut params = ioctl::params::default();

        match ioctl::getparams(pps_fd, &mut params) {
            Ok(_) => (),
            Err(_) => return Err("unable to set parameters".to_string()),
        };

        params.mode |= ioctl::CAPTUREASSERT;

        match ioctl::setparams(pps_fd, &mut params) {
            Ok(_) => (),
            Err(_) => return Err("unable to set parameters".to_string()),
        };
    }

    Ok(true)
}
