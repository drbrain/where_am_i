mod device;
mod error;
pub mod ioctl;
pub mod state;

pub use device::Device;
pub use error::Error;

use crate::timestamp::Timestamp;
use state::State;
use libc::c_int;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::thread;
use std::time::SystemTime;
use tracing::error;

pub struct PPS {
    shared_state: Arc<Mutex<State>>,
}

impl PPS {
    pub fn new(device: String, precision: i32, fd: c_int) -> Self {
        let state = State::new(device, precision, fd);

        let shared_state = Arc::new(Mutex::new(state));

        let thread_shared_state = shared_state.clone();

        thread::spawn(move || run(thread_shared_state));

        PPS { shared_state }
    }
}

fn run(shared_state: Arc<Mutex<State>>) {
    let mut shared_state = shared_state.lock().unwrap();

    // reset timestamp
    shared_state.result = None;

    fetch_pps(&mut shared_state);

    if let Some(waker) = shared_state.waker.take() {
        waker.wake()
    }
}

fn fetch_pps(shared_state: &mut State) {
    let mut data = ioctl::data::default();
    data.timeout.flags = ioctl::TIME_INVALID;

    let data_ptr: *mut ioctl::data = &mut data;
    let fetched;

    unsafe {
        fetched = ioctl::fetch(shared_state.fd, data_ptr);
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
            let device = shared_state.device.clone();
            let precision = shared_state.precision;

            let pps_obj = Timestamp::from_pps_time(device, precision, data, now);

            shared_state.result = Some(pps_obj);
        }
        Err(e) => {
            error!("unable to get system clock timestamp for PPS event ({:?})", e);
        }
    }
}

impl Future for PPS {
    type Output = Timestamp;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.shared_state.lock().unwrap();

        if let Some(pps_time) = state.result.as_ref() {
            Poll::Ready(pps_time.clone())
        } else {
            state.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}
