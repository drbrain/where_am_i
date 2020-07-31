mod ioctl;

use crate::JsonSender;

use serde_json::json;
use serde_json::Value;

use libc::c_int;

use std::error::Error;
use std::fmt;
use std::fs::OpenOptions;
use std::future::Future;
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
use tokio::sync::broadcast;

use tracing::error;
use tracing::info;

#[derive(Debug)]
pub struct PPS {
    pub name: String,
    pub tx: JsonSender,
    // Name of the device sent in PPS records
    device_name: String,
}

impl PPS {
    pub fn new(name: String, device_name: String) -> Self {
        let (tx, _) = broadcast::channel(5);

        PPS { name, tx, device_name }
    }

    #[tracing::instrument]
    fn open(&mut self) -> Result<File, Box<dyn Error>> {
        let pps = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.name.clone())?;

        info!("Opened {}", self.name);

        Ok(File::from_std(pps))
    }

    #[tracing::instrument]
    fn configure(&self, pps: File) -> Result<File, Box<dyn Error>> {
        let pps_fd = pps.as_raw_fd();

        unsafe {
            let mut mode = 0;

            if ioctl::getcap(pps_fd, &mut mode).is_err() {
                return Err(Box::new(PPSError::CapabilitiesFailed(self.name.clone())));
            };

            if mode & ioctl::CANWAIT == 0 {
                return Err(Box::new(PPSError::CannotWait(self.name.clone())));
            };

            if (mode & ioctl::CAPTUREASSERT) == 0 {
                return Err(Box::new(PPSError::CannotCaptureAssert(self.name.clone())));
            };

            let mut params = ioctl::params::default();

            if ioctl::getparams(pps_fd, &mut params).is_err() {
                return Err(Box::new(PPSError::CannotGetParameters(self.name.clone())));
            };

            params.mode |= ioctl::CAPTUREASSERT;

            if ioctl::setparams(pps_fd, &mut params).is_err() {
                return Err(Box::new(PPSError::CannotSetParameters(self.name.clone())));
            };
        }

        Ok(pps)
    }

    #[tracing::instrument]
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let file = self.open()?;
        let pps = self.configure(file)?;

        let name = self.name.clone();
        let tx = self.tx.clone();
        let device_name = self.device_name.clone();

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

                pps_data["device"] = device_name.clone().into();

                if let Err(_e) = tx.send(pps_data) {
                    // error!("send error: {:?}", e);
                }
            }
        });

        Ok(())
    }
}

#[derive(Debug)]
struct FetchTime {
    real_sec: i64,
    real_nsec: i32,
    clock_sec: u64,
    clock_nsec: u32,
}

#[derive(Debug)]
struct FetchState {
    result: Option<FetchTime>,
    ok: bool,
    completed: bool,
    waker: Option<Waker>,
}

struct FetchFuture {
    shared_state: Arc<Mutex<FetchState>>,
}

impl FetchFuture {
    pub fn new(fd: c_int) -> Self {
        let state = FetchState {
            result: None,
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
                Ok(_) => {
                    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
                    match now {
                        Ok(n) => {
                            let pps_obj = FetchTime {
                                real_sec: data.info.assert_tu.sec,
                                real_nsec: data.info.assert_tu.nsec,
                                clock_sec: n.as_secs(),
                                clock_nsec: n.subsec_nanos(),
                            };

                            shared_state.ok = true;

                            Some(pps_obj)
                        }
                        Err(e) => {
                            shared_state.ok = false;
                            error!("unable to get timestamp for PPS event ({:?})", e);

                            None
                        }
                    }
                }
                Err(e) => {
                    shared_state.ok = false;
                    error!("unable to get PPS event ({:?})", e);

                    None
                }
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
    type Output = Result<Value, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.shared_state.lock().unwrap();

        if guard.completed {
            let fetch_time = guard.result.as_ref().unwrap();

            if guard.ok {
                Poll::Ready(Ok(json!({
                    "class":      "PPS".to_string(),
                    "device":     "".to_string(),
                    "real_sec":   fetch_time.real_sec,
                    "real_nsec":  fetch_time.real_nsec,
                    "clock_sec":  fetch_time.clock_sec,
                    "clock_nsec": fetch_time.clock_nsec,
                    "precision":  -1,
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

#[derive(Debug)]
pub enum PPSError {
    CannotCaptureAssert(String),
    CannotGetParameters(String),
    CannotSetParameters(String),
    CannotWait(String),
    CapabilitiesFailed(String),
}

impl fmt::Display for PPSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PPSError::CannotCaptureAssert(n) => {
                write!(f, "cannot capture assert events for PPS device {}", n)
            }
            PPSError::CannotGetParameters(n) => {
                write!(f, "cannot get parameters for PPS device {}", n)
            }
            PPSError::CannotSetParameters(n) => {
                write!(f, "cannot set parameters for PPS device {}", n)
            }
            PPSError::CannotWait(n) => write!(f, "{} cannot wait for PPS events", n),
            PPSError::CapabilitiesFailed(n) => {
                write!(f, "unable to get capabilities of PPS device {}", n)
            }
        }
    }
}

impl Error for PPSError {}
