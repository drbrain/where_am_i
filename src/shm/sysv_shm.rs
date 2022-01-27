use crate::timestamp::Timestamp;
use anyhow::Context;
use anyhow::Result;
use std::io;
use std::mem;
use std::ptr::NonNull;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;
use tracing::trace;
use volatile_register::RW;

const NTPD_BASE: i32 = 0x4e545030;

#[repr(C)]
pub struct Time {
    pub mode: RW<i32>,
    pub count: RW<i32>,
    pub clock_sec: RW<i32>,
    pub clock_usec: RW<i32>,
    pub receive_sec: RW<i32>,
    pub receive_usec: RW<i32>,
    pub leap: RW<i32>,
    pub precision: RW<i32>,
    pub nsamples: RW<i32>,
    pub valid: RW<i32>,
    pub clock_nsec: RW<u32>,
    pub receive_nsec: RW<u32>,
    _dummy: [u8; 8],
}

macro_rules! write {
    ($time: ident, $field:ident) => {
        write!($time, $field, $field)
    };
    ($time: ident, $field:ident, $value:expr) => {
        (*$time).$field.write($value)
    };
}

macro_rules! update {
    ($time: ident, $field:ident, $ex:expr) => {
        (*$time).$field.modify($ex)
    };
}

macro_rules! read {
    ($time: ident, $field:ident) => {
        (*$time).$field.read()
    };
}

pub struct ShmTime {
    time: NonNull<Time>,
    pub unit: i32,
}

impl ShmTime {
    pub fn new(unit: i32) -> Result<Self> {
        let permissions = if unit <= 1 { 0o600 } else { 0o666 };

        let id = get_id(NTPD_BASE + unit, permissions)?;

        let shm;

        unsafe {
            shm = libc::shmat(id, std::ptr::null(), 0);
        }

        if -1 == shm as i32 {
            Err(io::Error::last_os_error())
                .with_context(|| format!("Unable to map shared memory id {}", unit))
        } else {
            let time = NonNull::new(shm as *mut Time).unwrap();

            Ok(ShmTime { time, unit })
        }
    }

    pub fn read(&self, last_count: i32) -> Option<crate::shm::Timestamp> {
        let time = self.time.as_ptr();
        let timestamp;

        unsafe {
            let count_before = read!(time, count);

            if count_before == last_count {
                return None;
            }

            compiler_fence(Ordering::SeqCst);

            let mode = read!(time, mode);
            let clock_sec = read!(time, clock_sec);
            let clock_usec = read!(time, clock_usec);
            let receive_sec = read!(time, receive_sec);
            let receive_usec = read!(time, receive_usec);
            let leap = read!(time, leap);
            let precision = read!(time, precision);
            let nsamples = read!(time, nsamples);
            let valid = read!(time, valid);
            let clock_nsec = read!(time, clock_nsec);
            let receive_nsec = read!(time, receive_nsec);

            compiler_fence(Ordering::SeqCst);

            let count_after = read!(time, count);

            if count_before != count_after {
                // We probably raced a clock write or NTP read.
                return None;
            }

            let count = count_after;

            timestamp = crate::shm::Timestamp {
                mode,
                count,
                clock_sec,
                clock_usec,
                receive_sec,
                receive_usec,
                leap,
                precision,
                nsamples,
                valid,
                clock_nsec,
                receive_nsec,
            };
        }

        trace!(
            "read NTP timestamp on unit {} count {}: {:?}",
            self.unit,
            timestamp.count,
            timestamp
        );

        Some(timestamp)
    }

    pub fn write(&mut self, ts: &Timestamp, precision: i32, leap: i32) {
        let time = self.time.as_ptr();

        // 2038 problem
        let reference_sec = ts.reference_sec.try_into().unwrap();
        let reference_nsec = ts.reference_nsec;
        let reference_usec = (reference_nsec / 1000) as i32;

        // 2038 problem
        let received_sec = ts.received_sec.try_into().unwrap();
        let received_nsec = ts.received_nsec;
        let received_usec = (received_nsec / 1000) as i32;

        let last_count;

        unsafe {
            write!(time, valid, 0);
            update!(time, count, |c| c + 1);

            compiler_fence(Ordering::SeqCst);

            write!(time, clock_sec, reference_sec);
            write!(time, clock_usec, reference_usec);

            write!(time, receive_sec, received_sec);
            write!(time, receive_usec, received_usec);

            write!(time, leap);

            write!(time, precision);

            write!(time, clock_nsec, reference_nsec);
            write!(time, receive_nsec, received_nsec);

            compiler_fence(Ordering::SeqCst);

            update!(time, count, |c| c + 1);
            write!(time, valid, 1);

            last_count = read!(time, count);
        }

        trace!(
            "set NTP timestamp on unit {} count {}: {}.{}",
            self.unit,
            last_count,
            ts.reference_sec,
            ts.reference_nsec
        );
    }
}

pub fn get_id(unit: i32, perms: i32) -> Result<i32> {
    let size = mem::size_of::<Time>();
    let flags = libc::IPC_CREAT | perms;

    let id;

    unsafe {
        id = libc::shmget(unit, size, flags);
    }

    if -1 == id {
        Err(io::Error::last_os_error())
            .with_context(|| format!("Unable to get shared memory id {}", unit))
    } else {
        Ok(id)
    }
}

impl Drop for ShmTime {
    fn drop(&mut self) {
        unsafe {
            let time = self.time.as_ptr();

            let ok = libc::shmdt(time as *const libc::c_void);

            if -1 == ok {
                let error = io::Error::last_os_error();
                panic!("unable to unmap shared memory ({:?})", error);
            }
        }
    }
}
