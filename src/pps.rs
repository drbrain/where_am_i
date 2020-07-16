mod ioctl;

use crate::JsonQueue;

use json::object;

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::time::SystemTime;

use tokio::fs::File;
use tokio::sync::broadcast;

pub async fn spawn(device: String) -> JsonQueue {
    let pps = match OpenOptions::new().read(true).write(true).open(&device) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error opening {}: {}", device, e);
            std::process::exit(1);
        }
    };

    eprintln!("Opened {}", device);
    let pps = File::from_std(pps);

    match configure(pps.as_raw_fd()) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}: {}", device, e);
            std::process::exit(1);
        }
    };

    let (tx, _) = broadcast::channel(5);
    let pps_tx = tx.clone();

    tokio::spawn(async move {
        let mut data = ioctl::data::default();
        let data_ptr: *mut ioctl::data = &mut data;

        loop {
            data.timeout.flags = ioctl::TIME_INVALID;

            unsafe {
                let result;
                result = ioctl::fetch(pps.as_raw_fd(), data_ptr);

                if result.is_err() {
                    let err = result.err().unwrap();
                    eprintln!("PPS fetch error on {} ({:?})", device, err);
                    continue;
                }
            };

            let received = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n,
                Err(_) => continue,
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

            match pps_tx.send(pps_obj) {
                Ok(_)  => (),
                Err(_) => (),
            }
        }
    });

    return tx
}

fn configure(pps_fd: i32) -> Result<bool, String> {
    unsafe {
        let mut current_mode = 0;
        let mut params = ioctl::params::default();

        let result = ioctl::getparams(pps_fd, &mut params);

        if result.is_err() {
            let err = result.err().unwrap();
            match err {
                nix::Error::Sys(enotty) => return Err("not a PPS device".to_string()),
                e => return Err(format!("other error {:?}", e)),
            };
        }

        let result = ioctl::getcap(pps_fd, &mut current_mode);

        if result.is_err() {
            let err = result.err().unwrap();
            return Err(format!("unable to get capabilities ({:?})", err));
        }

        if current_mode & ioctl::CANWAIT == 0 {
            return Err("cannot wait".to_string());
        }

        if (current_mode & ioctl::CAPTUREASSERT) == 0 {
            return Err("cannot capture asserts".to_string());
        }

        params.mode |= ioctl::CAPTUREASSERT;

        let result = ioctl::setparams(pps_fd, &mut params);

        if result.is_err() {
            let err = result.err().unwrap();
            return Err("unable to set parameters".to_string());
        }
    }

    Ok(true)
}
