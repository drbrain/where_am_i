mod device;
mod error;
pub mod ioctl;
pub mod state;

pub use device::Device;
pub use error::Error;

use crate::timestamp::Timestamp;
use libc::c_int;
use state::State;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread;
use std::time::SystemTime;
use tokio_stream::Stream;
use tracing::error;

pub struct PPS {
    pps_state: Arc<Mutex<State>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl PPS {
    pub fn new(device: String, precision: i32, fd: c_int) -> Self {
        let state = State::new(device, precision, fd);

        let pps_state = Arc::new(Mutex::new(state));
        let waker = Arc::new(Mutex::new(None));

        let thread_pps_state = pps_state.clone();
        let thread_pps_waker = waker.clone();

        thread::spawn(move || run(thread_pps_state, thread_pps_waker));

        PPS { pps_state, waker }
    }
}

fn run(pps_state: Arc<Mutex<State>>, waker: Arc<Mutex<Option<Waker>>>) {
    loop {
        let mut pps_state = pps_state.lock().unwrap();

        // reset timestamp
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

    unsafe {
        fetched = ioctl::fetch(pps_state.fd, data_ptr);
    }

    match fetched {
        Ok(_) => (),
        Err(e) => {
            error!("unable to get PPS event ({:?})", e);
            return;
        }
    }

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match now {
        Ok(now) => {
            let device = pps_state.device.clone();
            let precision = pps_state.precision;

            let pps_obj = Timestamp::from_pps_time(device, precision, data, now);

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
