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

    // reset shared state
    shared_state.ok = false;
    shared_state.result = None;

    fetch_pps(&mut shared_state);

    shared_state.completed = true;

    if let Some(waker) = shared_state.waker.take() {
        waker.wake()
    }
}

fn fetch_pps(shared_state: &mut State) {
    let mut data = ioctl::data::default();
    data.timeout.flags = ioctl::TIME_INVALID;

    let data_ptr: *mut ioctl::data = &mut data;
    let result;

    unsafe {
        result = ioctl::fetch(shared_state.fd, data_ptr);
    }

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match (result, now) {
        (Ok(_), Ok(n)) => {
            let device = shared_state.device.clone();
            let precision = shared_state.precision;

            let pps_obj = Timestamp::from_pps_time(device, precision, data, n);

            shared_state.ok = true;

            shared_state.result = Some(pps_obj);
        }
        (Ok(_), Err(e)) => {
            error!("unable to get timestamp for PPS event ({:?})", e);
        }
        (Err(e), _) => {
            error!("unable to get PPS event ({:?})", e);
        }
    }
}

impl Future for PPS {
    type Output = Result<Timestamp, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.shared_state.lock().unwrap();

        if guard.completed {
            let pps_time = guard.result.as_ref().unwrap();

            if guard.ok {
                Poll::Ready(Ok(pps_time.clone()))
            } else {
                Poll::Ready(Err("something went wrong".to_string()))
            }
        } else {
            guard.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}
