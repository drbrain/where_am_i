use crate::TSReceiver;
use crate::TSSender;
use crate::Timestamp;
use crate::TimestampKind;

use crate::shm::sysv_shm;
use crate::shm::sysv_shm::ShmTime;

use std::convert::TryInto;
use std::io;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::time::sleep;

use tracing::error;
use tracing::trace;

pub struct NtpShm {}

const NTPD_BASE: i32 = 0x4e545030;

impl NtpShm {
    pub async fn relay(unit: i32, rx: TSReceiver) {
        tokio::spawn(relay_timestamps(unit, rx));
    }

    pub async fn watch(unit: i32, device: String, tx: TSSender) {
        tokio::spawn(watch_timestamps(unit, device, tx));
    }
}

fn map_ntp_unit(unit: i32) -> io::Result<ShmTime> {
    let permissions = if unit <= 1 { 0o600 } else { 0o666 };

    let id = sysv_shm::get_id(NTPD_BASE + unit, permissions)?;

    sysv_shm::map(id)
}

async fn relay_timestamps(unit: i32, mut rx: TSReceiver) {
    let mut time = map_ntp_unit(unit).unwrap();
    let mut last_count: i32;

    while let Ok(ts) = rx.recv().await {
        let reference_sec = ts.reference_sec.try_into().unwrap_or(0);
        let reference_nsec = ts.reference_nsec;
        let reference_usec = (reference_nsec / 1000) as i32;

        let received_sec = ts.received_sec.try_into().unwrap_or(0);
        let received_nsec = ts.received_nsec;
        let received_usec = (received_nsec / 1000) as i32;

        let leap = ts.leap;
        let precision = ts.precision;

        time.map_mut(|t| &mut t.valid).write(0);
        time.map_mut(|t| &mut t.count).update(|c| *c += 1);

        compiler_fence(Ordering::SeqCst);

        time.map_mut(|t| &mut t.clock_sec).write(reference_sec);
        time.map_mut(|t| &mut t.clock_usec).write(reference_usec);

        time.map_mut(|t| &mut t.receive_sec).write(received_sec);
        time.map_mut(|t| &mut t.receive_usec).write(received_usec);

        time.map_mut(|t| &mut t.leap).write(leap);

        time.map_mut(|t| &mut t.precision).write(precision);

        time.map_mut(|t| &mut t.clock_nsec).write(reference_nsec);
        time.map_mut(|t| &mut t.receive_nsec).write(received_nsec);

        compiler_fence(Ordering::SeqCst);

        time.map_mut(|t| &mut t.count).update(|c| *c += 1);
        time.map_mut(|t| &mut t.valid).write(1);
        last_count = time.map(|t| &t.count).read();

        trace!("set NTP timestamp on unit {} count {}", unit, last_count);
    }

    error!("Sending timestamps failed");

    sysv_shm::unmap(time);
}

// NTP reads the shared memory as described at http://doc.ntp.org/4.2.8/drivers/driver28.html
//
// In mode 1 it resets valid and bumps count after reading values.  We can't trust valid as it may
// change while we're reading values.
//
// Instead we make a best-effort by tracking count.  If it is different than last go-around and
// did not change while reading we probably got new values, so we report them.
async fn watch_timestamps(unit: i32, device: String, tx: TSSender) {
    let time = map_ntp_unit(unit).unwrap();
    let mut last_count: i32 = 0;

    loop {
        let count_before = time.map(|t| &t.count).read();

        if count_before == last_count {
            sleep(Duration::from_millis(10)).await;
            continue;
        }

        compiler_fence(Ordering::SeqCst);

        let reference_sec = time.map(|t| &t.clock_sec).read();
        let reference_nsec = time.map(|t| &t.clock_nsec).read();

        let received_sec = time.map(|t| &t.receive_sec).read();
        let received_nsec = time.map(|t| &t.receive_nsec).read();

        let leap = time.map(|t| &t.leap).read();
        let precision = time.map(|t| &t.precision).read();

        compiler_fence(Ordering::SeqCst);

        let count_after = time.map(|t| &t.count).read();

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

        let timestamp = Timestamp {
            device: device.clone(),
            kind: TimestampKind::GPS, // TODO pass in type somewhere
            precision,
            leap,
            received_sec: received_sec.try_into().unwrap_or(0),
            received_nsec,
            reference_sec: reference_sec.try_into().unwrap_or(0),
            reference_nsec,
        };

        trace!(
            "detected NTP timestamp on unit {} count {}: {:?}",
            unit,
            last_count,
            timestamp
        );

        if tx.send(timestamp).is_ok() {};

        sleep(Duration::from_millis(1000)).await;
    }
}
