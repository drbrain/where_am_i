use crate::JsonReceiver;
use crate::JsonSender;

use crate::shm::sysv_shm;
use crate::shm::sysv_shm::ShmTime;

use std::convert::TryInto;
use std::error::Error;
use std::io;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;

#[derive(Debug)]
pub struct NtpShm {
    gps_tx: JsonSender,
    pps_tx: JsonSender,
    unit: i32,
}

const NTPD_BASE: i32 = 0x4e545030;

impl NtpShm {
    pub fn new(
        unit: i32,
        gps_tx: JsonSender,
        pps_tx: JsonSender,
    ) -> Result<NtpShm, Box<dyn Error>> {
        Ok(NtpShm {
            gps_tx,
            pps_tx,
            unit,
        })
    }

    pub async fn relay_timestamps(&self) {
        let gps_rx = self.gps_tx.subscribe();
        let gps_unit = self.unit;

        tokio::spawn(relay_timestamps(gps_rx, gps_unit));

        let pps_rx = self.pps_tx.subscribe();
        let pps_unit = self.unit + 1;

        tokio::spawn(relay_timestamps(pps_rx, pps_unit));
    }
}

fn map_ntp_unit(unit: i32) -> io::Result<ShmTime> {
    let permissions = if unit <= 1 { 0o600 } else { 0o666 };

    let id = sysv_shm::get_id(NTPD_BASE + unit, permissions)?;

    sysv_shm::map(id)
}

async fn relay_timestamps(mut rx: JsonReceiver, unit: i32) {
    let mut time = map_ntp_unit(unit).unwrap();

    loop {
        let gps_ts = match rx.recv().await {
            Ok(ts) => ts,
            Err(_) => break,
        };

        let clock_sec = gps_ts["clock_sec"]
            .as_i64()
            .unwrap_or(0)
            .try_into()
            .unwrap();
        let clock_nsec = gps_ts["clock_nsec"]
            .as_i64()
            .unwrap_or(0)
            .try_into()
            .unwrap();
        let clock_usec = (clock_nsec / 1000) as i32;
        let receive_sec = gps_ts["real_sec"].as_i64().unwrap_or(0).try_into().unwrap();
        let receive_nsec = gps_ts["real_nsec"]
            .as_i64()
            .unwrap_or(0)
            .try_into()
            .unwrap();
        let receive_usec = (receive_nsec / 1000) as i32;

        time.valid.write(0);
        time.count.update(|c| *c += 1);

        compiler_fence(Ordering::SeqCst);

        time.clock_sec = clock_sec;
        time.clock_usec = clock_usec;
        time.receive_sec = receive_sec;
        time.receive_usec = receive_usec;
        time.leap = 0;
        time.precision = -1;
        time.clock_nsec = clock_nsec;
        time.receive_nsec = receive_nsec;

        compiler_fence(Ordering::SeqCst);

        time.count.update(|c| *c += 1);
        time.valid.write(1);
    }

    sysv_shm::unmap(time);
}
