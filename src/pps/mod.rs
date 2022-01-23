mod device;
pub mod ioctl;
pub mod state;

pub use device::Device;

use crate::timestamp::Timestamp;
use anyhow::anyhow;
use anyhow::Result;
use libc::c_int;
use state::State;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread;
use std::time::SystemTime;
use tokio::fs::File;
use tokio_stream::Stream;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;

#[derive(Debug)]
pub struct PPS {
    // Don't let the File go out of scope
    _pps_file: File,
    pps_state: Arc<Mutex<State>>,
    waker: Arc<Mutex<Option<Waker>>>,
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

        let pps_state = Arc::new(Mutex::new(state));
        let waker = Arc::new(Mutex::new(None));

        let thread_pps_state = pps_state.clone();
        let thread_pps_waker = waker.clone();

        thread::spawn(move || run(thread_pps_state, thread_pps_waker));
        info!("Started PPS device {}", &device_name);

        Ok(PPS {
            _pps_file: pps_file,
            pps_state,
            waker,
        })
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

fn run(pps_state: Arc<Mutex<State>>, waker: Arc<Mutex<Option<Waker>>>) {
    loop {
        // reset timestamp
        let mut pps_state = pps_state.lock().unwrap();
        pps_state.result = None;

        fetch_pps(&mut pps_state);

        let mut waker = waker.lock().unwrap();

        if let Some(waker) = waker.take() {
            waker.wake()
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

impl Stream for PPS {
    type Item = Timestamp;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let state = self.pps_state.lock().unwrap();

        if let Some(pps_time) = state.result.as_ref() {
            Poll::Ready(Some(pps_time.clone()))
        } else {
            let mut waker = self.waker.lock().unwrap();

            *waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, Some(1))
    }
}
