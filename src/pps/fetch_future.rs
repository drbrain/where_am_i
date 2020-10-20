use crate::pps::ioctl;

use serde_json::json;
use serde_json::Value;

use libc::c_int;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use tracing::error;

#[derive(Debug)]
pub struct FetchTime {
    pub device: String,
    pub real_sec: i64,
    pub real_nsec: i32,
    pub clock_sec: u64,
    pub clock_nsec: u32,
    pub precision: i32,
}

impl FetchTime {
    fn new(state: &FetchState, pps_time: ioctl::data, now: Duration) -> Self {
        FetchTime {
            device: state.device.clone(),
            real_sec: pps_time.info.assert_tu.sec,
            real_nsec: pps_time.info.assert_tu.nsec,
            clock_sec: now.as_secs(),
            clock_nsec: now.subsec_nanos(),
            precision: state.precision,
        }
    }
}

#[derive(Debug)]
struct FetchState {
    device: String,
    precision: i32,
    fd: c_int,
    result: Option<FetchTime>,
    ok: bool,
    completed: bool,
    waker: Option<Waker>,
}

impl FetchState {
    fn new(device: String, precision: i32, fd: c_int) -> Self {
        FetchState {
            device,
            precision,
            fd,
            result: None,
            ok: false,
            completed: false,
            waker: None,
        }
    }
}

pub struct FetchFuture {
    shared_state: Arc<Mutex<FetchState>>,
}

impl FetchFuture {
    pub fn new(device: String, precision: i32, fd: c_int) -> Self {
        let state = FetchState::new(device, precision, fd);

        let shared_state = Arc::new(Mutex::new(state));

        let thread_shared_state = shared_state.clone();

        thread::spawn(move || run(thread_shared_state));

        FetchFuture { shared_state }
    }
}

fn run(shared_state: Arc<Mutex<FetchState>>) {
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

fn fetch_pps(shared_state: &mut FetchState) {
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
            let pps_obj = FetchTime::new(shared_state, data, n);

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

impl Future for FetchFuture {
    type Output = Result<Value, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.shared_state.lock().unwrap();

        if guard.completed {
            let fetch_time = guard.result.as_ref().unwrap();

            if guard.ok {
                Poll::Ready(Ok(json!({
                    "class":      "PPS".to_string(),
                    "device":     fetch_time.device,
                    "real_sec":   fetch_time.real_sec,
                    "real_nsec":  fetch_time.real_nsec,
                    "clock_sec":  fetch_time.clock_sec,
                    "clock_nsec": fetch_time.clock_nsec,
                    "precision":  fetch_time.precision,
                })))
            } else {
                Poll::Ready(Err("something went wrong".to_string()))
            }
        } else {
            guard.waker = Some(cx.waker().clone());

            Poll::Pending
        }
    }
}
