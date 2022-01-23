pub mod ioctl;
pub mod state;

use crate::timestamp::Timestamp;
use anyhow::anyhow;
use anyhow::Result;
use libc::c_int;
use state::State;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::sync::watch;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;

#[derive(Clone, Debug)]
pub struct PPS {
    // Don't let the File go out of scope
    _pps_file: Arc<File>,
    current_timestamp: watch::Receiver<Option<Timestamp>>,
}

impl PPS {
    pub fn new(device_name: String) -> Result<Self> {
        let pps_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(device_name.clone())?;

        let pps_file = File::from_std(pps_file);
        let fd = pps_file.as_raw_fd();
        debug!("Opened PPS {} as fd ({})", &device_name, fd);

        configure(fd, &device_name)?;

        let state = State::new(device_name.clone(), fd);
        let (sender, current_timestamp) = watch::channel(None);

        let thread_device_name = device_name.clone();

        tokio::task::spawn_blocking(move || {
            run(state, sender);
            trace!("PPS {} shutdown, no more watchers", &thread_device_name);
        });

        info!("Started PPS device {}", &device_name);

        Ok(PPS {
            _pps_file: Arc::new(pps_file),
            current_timestamp: current_timestamp,
        })
    }

    pub fn current_timestamp(&self) -> watch::Receiver<Option<Timestamp>> {
        self.current_timestamp.clone()
    }
}

fn configure(pps_fd: c_int, name: &str) -> Result<()> {
    unsafe {
        let mut mode = 0;

        if let Err(e) = ioctl::getcap(pps_fd, &mut mode) {
            return Err(anyhow!("cannot capture PPS assert for {} ({})", name, e));
        };
        trace!("PPS {} mode: {}", name, mode);

        if mode & ioctl::CANWAIT == 0 {
            return Err(anyhow!("PPS device {} can't wait", name));
        };
        trace!("PPS {} can wait", name);

        if (mode & ioctl::CAPTUREASSERT) == 0 {
            return Err(anyhow!("PPS device {} can't capture assert", name));
        };
        trace!("PPS {} can capture assert", name);

        let mut params = ioctl::params::default();

        if let Err(e) = ioctl::getparams(pps_fd, &mut params) {
            return Err(anyhow!("cannot get PPS parameters for {} ({})", name, e));
        };
        trace!("PPS {} params: {:?}", name, params);

        params.mode |= ioctl::CAPTUREASSERT;

        if let Err(e) = ioctl::setparams(pps_fd, &mut params) {
            return Err(anyhow!("cannot set PPS parameters for {} ({})", name, e));
        };
        trace!("Set PPS {} params {:?}", name, params);
    }

    trace!("PPS {} configured", name);
    Ok(())
}

fn run(mut state: State, sender: watch::Sender<Option<Timestamp>>) {
    loop {
        // reset timestamp
        state.result = None;

        fetch_pps(&mut state);

        if let Err(_) = sender.send(state.result) {
            error!("No more PPS receivers");
            return;
        }
    }
}

fn fetch_pps(pps_state: &mut State) {
    let mut data = ioctl::data::default();
    data.timeout.flags = ioctl::TIME_INVALID;

    let data_ptr: *mut ioctl::data = &mut data;
    let fetched;
    trace!("Waiting for PPS signal for fd {}", pps_state.fd);

    unsafe {
        fetched = ioctl::fetch(pps_state.fd, data_ptr);
    }

    match fetched {
        Ok(_) => (),
        Err(e) => {
            error!("unable to get PPS event from fd {} ({:?})", pps_state.fd, e);
            return;
        }
    }

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
    trace!("Received PPS signal from fd {}", pps_state.fd);

    match now {
        Ok(now) => {
            let pps_obj = Timestamp::from_pps_time(data, now);

            pps_state.result = Some(pps_obj);
        }
        Err(e) => {
            error!(
                "unable to get system clock timestamp for PPS event ({:?})",
                e
            );
        }
    }
}
