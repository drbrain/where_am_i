use crate::shm::sysv_shm;
use crate::shm::sysv_shm::ShmTime;
use crate::TSReceiver;
use std::convert::TryInto;
use std::io;
use std::ops::Deref;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tokio::time::sleep;
use tracing::error;
use tracing::trace;

pub struct NtpShm {
    unit: i32,
}

const NTPD_BASE: i32 = 0x4e545030;

impl NtpShm {
    pub fn new(unit: i32) -> Self {
        NtpShm { unit }
    }

    pub async fn relay(&self, leap: i32, precision: i32, rx: TSReceiver) {
        tokio::spawn(relay_timestamps(self.unit, precision, leap, rx));
    }

    pub async fn relay_pps(
        &self,
        current_precision: watch::Receiver<i32>,
        leap: i32,
        current_timestamp: watch::Receiver<Option<crate::timestamp::Timestamp>>,
    ) {
        tokio::spawn(relay_pps_timestamps(
            self.unit,
            current_precision,
            leap,
            current_timestamp,
        ));
    }

    pub async fn watch(
        unit: i32,
        device: String,
        tx: broadcast::Sender<(String, crate::shm::Timestamp)>,
    ) {
        tokio::spawn(watch_timestamps(unit, device, tx));
    }
}

fn map_ntp_unit(unit: i32) -> io::Result<ShmTime> {
    let permissions = if unit <= 1 { 0o600 } else { 0o666 };

    let id = sysv_shm::get_id(NTPD_BASE + unit, permissions)?;

    sysv_shm::map(id)
}

async fn relay_timestamps(unit: i32, precision: i32, leap: i32, mut rx: TSReceiver) {
    let mut shm_time = map_ntp_unit(unit).unwrap();

    while let Ok(ts) = rx.recv().await {
        write_timestamp(&mut shm_time, &ts, precision, leap);
    }

    error!("Sending timestamps failed");

    sysv_shm::unmap(shm_time);
}

async fn relay_pps_timestamps(
    unit: i32,
    current_precision: watch::Receiver<i32>,
    leap: i32,
    mut current_timestamp: watch::Receiver<Option<crate::timestamp::Timestamp>>,
) {
    let mut shm_time = map_ntp_unit(unit).unwrap();

    loop {
        if let Err(_) = current_timestamp.changed().await {
            error!("PPS source for NTP shm unit {} shut down", unit);
            break;
        }

        let precision = *current_precision.borrow().deref();

        if let Some(ts) = current_timestamp.borrow().deref() {
            write_timestamp(&mut shm_time, ts, precision, leap);
        }
    }

    sysv_shm::unmap(shm_time);
}

macro_rules! write {
    ($time: ident, $field:ident) => {
        write!($time, $field, $field)
    };
    ($time: ident, $shm_field:ident, $value:expr) => {
        $time.map_mut(|t| &mut t.$shm_field).write($value)
    };
}

macro_rules! update {
    ($time: ident, $shm_field:ident, $ex:expr) => {
        $time.map_mut(|t| &mut t.$shm_field).update($ex)
    };
}

macro_rules! read {
    ($time: ident, $field:ident) => {
        $time.map(|t| &t.$field).read()
    };
}

fn write_timestamp(
    time: &mut ShmTime,
    ts: &crate::timestamp::Timestamp,
    precision: i32,
    leap: i32,
) -> i32 {
    let reference_sec = ts.reference_sec.try_into().unwrap_or(0);
    let reference_nsec = ts.reference_nsec;
    let reference_usec = (reference_nsec / 1000) as i32;

    let received_sec = ts.received_sec.try_into().unwrap_or(0);
    let received_nsec = ts.received_nsec;
    let received_usec = (received_nsec / 1000) as i32;

    write!(time, valid, 0);
    update!(time, count, |c| *c += 1);

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

    update!(time, count, |c| *c += 1);
    write!(time, valid, 1);

    let last_count: i32 = read!(time, count);

    trace!(
        "set NTP timestamp {}: {}.{}",
        last_count,
        ts.reference_sec,
        ts.reference_nsec
    );

    last_count
}

// NTP reads the shared memory as described at http://doc.ntp.org/4.2.8/drivers/driver28.html
//
// In mode 1 it resets valid and bumps count after reading values.  We can't trust valid as it may
// change while we're reading values.
//
// Instead we make a best-effort by tracking count.  If it is different than last go-around and
// did not change while reading we probably got new values, so we report them.
async fn watch_timestamps(
    unit: i32,
    device: String,
    tx: broadcast::Sender<(String, crate::shm::Timestamp)>,
) {
    let time = map_ntp_unit(unit).unwrap();
    let mut last_count: i32 = 0;

    // TODO use tokio::time::interval
    loop {
        let count_before = read!(time, count);

        if count_before == last_count {
            sleep(Duration::from_millis(10)).await;
            continue;
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
            //
            // If from a write we might bail again if we race an NTP read on our next try.
            //
            // If from a read then we'll have stable values on our next try.
            last_count = count_before;

            sleep(Duration::from_millis(10)).await;
            continue;
        }

        last_count = count_after;
        let count = count_after;

        let timestamp = crate::shm::Timestamp {
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

        trace!(
            "detected NTP timestamp on unit {} count {}: {:?}",
            unit,
            last_count,
            timestamp
        );

        if let Err(_) = tx.send((device.clone(), timestamp)) {
            break;
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
