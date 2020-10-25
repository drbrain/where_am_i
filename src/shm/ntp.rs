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

use tokio::time::delay_for;

use tracing::error;

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

    while let Ok(ts) = rx.recv().await {
        let clock_sec = ts.clock_sec.try_into().unwrap_or(0);
        let clock_nsec = ts.clock_nsec;
        let clock_usec = (clock_nsec / 1000) as i32;

        let receive_sec = ts.real_sec.try_into().unwrap_or(0);
        let receive_nsec = ts.real_nsec.try_into().unwrap_or(0);
        let receive_usec = (receive_nsec / 1000) as i32;

        let leap = ts.leap;
        let precision = ts.precision;

        time.valid.write(0);
        time.count.update(|c| *c += 1);

        compiler_fence(Ordering::SeqCst);

        time.clock_sec = clock_sec;
        time.clock_usec = clock_usec;

        time.receive_sec = receive_sec;
        time.receive_usec = receive_usec;

        time.leap = leap;

        time.precision = precision;

        time.clock_nsec = clock_nsec;
        time.receive_nsec = receive_nsec;

        compiler_fence(Ordering::SeqCst);

        time.count.update(|c| *c += 1);
        time.valid.write(1);
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
        let count_before = time.count.read();

        if count_before == last_count {
            delay_for(Duration::from_millis(10)).await;
            continue;
        }

        compiler_fence(Ordering::SeqCst);

        let clock_sec = time.clock_sec;
        let clock_nsec = time.clock_nsec;

        let real_sec = time.receive_sec;
        let real_nsec = time.receive_nsec;

        let leap = time.leap;
        let precision = time.precision;

        compiler_fence(Ordering::SeqCst);

        let count_after = time.count.read();

        if count_before != count_after {
            // We probably raced a clock write or NTP read.
            //
            // If from a write we might bail again if we race an NTP read on our next try.
            //
            // If from a read then we'll have stable values on our next try.
            last_count = count_before;

            delay_for(Duration::from_millis(10)).await;
            continue;
        }

        last_count = count_after;

        let timestamp = Timestamp {
            device: device.clone(),
            kind: TimestampKind::GPS, // TODO pass in type somewhere
            precision: precision,
            leap: leap,
            real_sec: real_sec.into(),
            real_nsec: real_nsec.try_into().unwrap_or(0),
            clock_sec: clock_sec.try_into().unwrap_or(0),
            clock_nsec: clock_nsec,
        };

        if tx.send(timestamp).is_ok() {};

        delay_for(Duration::from_millis(1000)).await;
    }
}
