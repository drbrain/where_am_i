mod ioctl;

use crate::JsonSender;

use json::object;
use json::stringify;

use libc::c_int;

use std::fs::OpenOptions;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

use tokio::fs::File;

use tracing::error;
use tracing::info;

#[tracing::instrument]
pub fn spawn(device: String, tx: JsonSender) -> Result<(), String> {
    let pps = match OpenOptions::new().read(true).write(true).open(&device) {
        Ok(p) => p,
        Err(e) => {
            return Err(format!("Error opening PPS {} ({})", device, e))
        }
    };

    info!("Opened {}", device);
    let pps = File::from_std(pps);

    match configure(pps.as_raw_fd()) {
        Ok(_) => (),
        Err(e) => {
            return Err(format!("configuring PPS device ({:?})", e))
        }
    };

    tokio::spawn(async move {
        info!("watching PPS events on {}", device);

        loop {
            let pps_obj = match FetchFuture::new(pps.as_raw_fd()).await {
                Ok(o) => o,
                Err(e) => {
                    error!("fetch error on {} ({:?})", device, e);
                    continue;
                }
            };

            match tx.send(pps_obj) {
                Ok(_)  => (),
                Err(e) => (), // error!("send error: {:?}", e),
            }
        }
    });

    Ok(())
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

#[derive(Debug)]
struct FetchState {
    result:    String,
    ok:        bool,
    completed: bool,
    waker:     Option<Waker>,
}

struct FetchFuture {
    shared_state: Arc<Mutex<FetchState>>,
}

impl FetchFuture {
    pub fn new(fd: c_int) -> Self {
        let state = FetchState {
            result: "incomplete".to_string(),
            ok: false,
            completed: false,
            waker: None,
        };

        let shared_state = Arc::new(Mutex::new(state));

        let thread_shared_state = shared_state.clone();

        thread::spawn(move || {
            let mut shared_state = thread_shared_state.lock().unwrap();

            let mut data = ioctl::data::default();
            data.timeout.flags = ioctl::TIME_INVALID;

            let data_ptr: *mut ioctl::data = &mut data;
            let result;

            unsafe {
                result = ioctl::fetch(fd, data_ptr);
            }

            shared_state.result = match result {
                Ok(_)  => {
                    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
                    match now {
                        Ok(n) => {
                            let pps_obj = object! {
                                class:      "PPS".to_string(),
                                device:     "".to_string(),
                                real_sec:   data.info.assert_tu.sec,
                                real_nsec:  data.info.assert_tu.nsec,
                                clock_sec:  n.as_secs(),
                                clock_nsec: n.subsec_nanos(),
                                precision:  -1,
                            };

                            shared_state.ok = true;

                            stringify(pps_obj)
                        },
                        Err(e) => {
                            shared_state.ok = false;

                            format!("unable to get timestamp for PPS event ({:?})", e)
                        },
                    }

                },
                Err(e) => {
                    shared_state.ok = false;
                    format!("unable to get PPS event ({:?})", e)
                },
            };

            shared_state.completed = true;

            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        });

        FetchFuture { shared_state }
    }
}

impl Future for FetchFuture {
    type Output = Result<String, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.shared_state.lock().unwrap();

        if guard.completed {
            let result = guard.result.to_string();

            match guard.ok {
                true  => Poll::Ready(Ok(result)),
                false => Poll::Ready(Err(result)),
            }
        } else {
            guard.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

