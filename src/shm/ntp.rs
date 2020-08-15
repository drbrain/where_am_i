use crate::JsonReceiver;

use crate::shm::sysv_shm;
use crate::shm::sysv_shm::ShmTime;

use std::convert::TryInto;
use std::io;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;

use tracing::error;

pub struct NtpShm {}

const NTPD_BASE: i32 = 0x4e545030;

impl NtpShm {
    pub async fn run(unit: i32, precision: i32, rx: JsonReceiver) {
        tokio::spawn(relay_timestamps(unit, precision, rx));
    }
}

fn map_ntp_unit(unit: i32) -> io::Result<ShmTime> {
    let permissions = if unit <= 1 { 0o600 } else { 0o666 };

    let id = sysv_shm::get_id(NTPD_BASE + unit, permissions)?;

    sysv_shm::map(id)
}

async fn relay_timestamps(unit: i32, precision: i32, mut rx: JsonReceiver) {
    let mut time = map_ntp_unit(unit).unwrap();

    while let Ok(ts) = rx.recv().await {
        let class = &ts["class"];

        if class != "TOFF" && class != "PPS" {
            continue;
        }

        let clock_sec = ts["clock_sec"].as_i64().unwrap_or(0).try_into().unwrap();
        let clock_nsec = ts["clock_nsec"].as_i64().unwrap_or(0).try_into().unwrap();
        let clock_usec = (clock_nsec / 1000) as i32;
        let receive_sec = ts["real_sec"].as_i64().unwrap_or(0).try_into().unwrap();
        let receive_nsec = ts["real_nsec"].as_i64().unwrap_or(0).try_into().unwrap();
        let receive_usec = (receive_nsec / 1000) as i32;

        time.valid.write(0);
        time.count.update(|c| *c += 1);

        compiler_fence(Ordering::SeqCst);

        time.clock_sec = clock_sec;
        time.clock_usec = clock_usec;
        time.receive_sec = receive_sec;
        time.receive_usec = receive_usec;
        time.leap = 0;
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
