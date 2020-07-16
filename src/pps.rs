use crate::JsonQueue;

use json::object;

use libc::c_int;

use std::fs::OpenOptions;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::time::SystemTime;

// use nix::sys::ioctl_read;

use tokio::fs::File;
use tokio::sync::broadcast;

#[derive(Default)]
#[repr(C)]
pub struct data {
    info:    info,
    timeout: time,
}

#[derive(Default)]
#[repr(C)]
pub struct info {
    assert_sequence: u32,   // sequence number of assert event
    clear_sequence:  u32,   // sequence number of clear event
    assert_tu:       time, // time of assert event
    clear_tu:        time, // time of clear event
    current_mode:    i32,   // current mode
}

#[derive(Default)]
#[repr(C)]
pub struct params {
    api_version:   i32,   // API version
    mode:          i32,   // current mode
    assert_off_tu: time, // assert offset compensation
    clear_off_tu:  time, // clear offset compensation
}

#[derive(Default)]
#[repr(C)]
pub struct time {
    sec:   i64, // seconds
    nsec:  i32, // nanoseconds
    flags: u32, // flags
}

const TIME_INVALID: u32 = 1<<0;

const CAPTUREASSERT: i32 = 0x01;   // capture assert events
const CAPTURECLEAR:  i32 = 0x02;   // capture clear events
const CAPTUREBOTH:   i32 = 0x03;   // capture both event types

const OFFSETASSERT:  i32 = 0x10;   // apply compensation for assert event
const OFFSETCLEAR:   i32 = 0x20;   // apply compensation for clear event

const ECHOASSERT:    i32 = 0x40;   // feed back assert event to output
const ECHOCLEAR:     i32 = 0x80;   // feed back clear event to output

const CANWAIT:       i32 = 0x100;  // Can we wait for an event?
const CANPOLL:       i32 = 0x200;  // Reserved

const DSFMT_TSPEC:   i32 = 0x1000; // struct timespec format
const DSFMT_NTPFP:   i32 = 0x2000; // NTP time format

const MAGIC: u8 = b'p';

const GETPARAMS: u8 = 0xa1;
const SETPARAMS: u8 = 0xa2;
const GETCAP:    u8 = 0xa3;
const FETCH:     u8 = 0xa4;

// ioctl_read!(getparams, MAGIC, GETPARAMS, params);
pub unsafe fn getparams(fd: c_int, data: *mut params) -> nix::Result<c_int> {
    let res = libc::ioctl(fd, request_code_read!(MAGIC, GETPARAMS, mem::size_of::<*mut params>()), data);
    nix::errno::Errno::result(res)
}
// ioctl_write_ptr!(setparams, MAGIC, SETPARAMS, params);
pub unsafe fn setparams(fd: c_int, data: *mut params) -> nix::Result<c_int> {
    let res = libc::ioctl(fd, request_code_write!(MAGIC, SETPARAMS, mem::size_of::<*mut params>()), data);
    nix::errno::Errno::result(res)
}

ioctl_read!(getcap, MAGIC, GETCAP, i32);

//ioctl_readwrite!(fetch, MAGIC, FETCH, data);
pub unsafe fn fetch(fd: c_int, data: *mut data) -> nix::Result<c_int> {
    let res = libc::ioctl(fd, request_code_readwrite!(MAGIC, FETCH, mem::size_of::<*mut data>()), data);
    nix::errno::Errno::result(res)
}

pub async fn spawn(device: String) -> JsonQueue {
    let mode = CAPTUREASSERT;
    let mut current_mode = 0;

    let pps = match OpenOptions::new().read(true).write(true).open(&device) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error opening {}: {}", device, e);
            std::process::exit(1);
        }
    };

    eprintln!("Opened {}", device);
    let pps = File::from_std(pps);

    unsafe {
        let mut params = params::default();

        let result = getparams(pps.as_raw_fd(), &mut params);

        if result.is_err() {
            let err = result.err().unwrap();
            match err {
                nix::Error::Sys(enotty) =>
                    eprintln!("{} is not a PPS device ({:?})", device, err),
                _ =>
                    eprintln!("{} other error ({:?})", device, err),
            };

            std::process::exit(1);
        }

        let result = getcap(pps.as_raw_fd(), &mut current_mode);

        if current_mode & CANWAIT == 0 {
            eprintln!("PPS device {} does not support waiting for pulse", device);
            std::process::exit(1);
        }

        if result.is_err() {
            let err = result.err().unwrap();
            eprintln!("Unable to get capabilities of {} ({:?})", device, err);
            std::process::exit(1);
        }

        if (current_mode & mode) == mode {
            let result = getparams(pps.as_raw_fd(), &mut params);

            if result.is_err() {
                let err = result.err().unwrap();
                eprintln!("Unable to get parameters of {} ({:?})", device, err);
                std::process::exit(1);
            }

            params.mode |= mode;

            let result = setparams(pps.as_raw_fd(), &mut params);

            if result.is_err() {
                let err = result.err().unwrap();
                eprintln!("Unable to set parameters of {} ({:?})", device, err);
                std::process::exit(1);
            }
        } else {
            eprintln!("Unable to set mode of {} to {:#06x}, {:#06x}", device, mode, current_mode);
            std::process::exit(1);
        }
    };

    //if 0 != result {
    //    eprintln!("{} is not a PPS device ({})", device, result);
    //    std::process::exit(1);
    //}

    let (tx, _) = broadcast::channel(5);
    let pps_tx = tx.clone();

    tokio::spawn(async move {
        let mut data = data::default();
        let data_ptr: *mut data = &mut data;

        loop {
            data.timeout.flags = TIME_INVALID;

            unsafe {
                let result;
                result = fetch(pps.as_raw_fd(), data_ptr);

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
