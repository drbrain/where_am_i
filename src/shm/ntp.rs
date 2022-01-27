use crate::shm::sysv_shm::ShmTime;
use crate::timestamp::Timestamp;
use anyhow::Result;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::watch;
use tokio::time::interval;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tracing::error;

pub struct NtpShm {
    shm_time: Arc<Mutex<ShmTime>>,
}

impl NtpShm {
    pub fn new(unit: i32) -> Result<Self> {
        let shm_time = Arc::new(Mutex::new(ShmTime::new(unit)?));

        Ok(NtpShm { shm_time })
    }

    // TODO make leap a watch::Receiver<i32>
    pub async fn update(
        &mut self,
        current_precision: &watch::Receiver<i32>,
        leap: i32,
        current_timestamp: &mut watch::Receiver<Timestamp>,
    ) {
        if let Err(_) = current_timestamp.changed().await {
            let guard = self.shm_time.lock().unwrap();
            error!("PPS source for NTP shm unit {} shut down", guard.unit);
            return;
        }

        let precision = *current_precision.borrow().deref();

        let ts = current_timestamp.borrow();
        let mut time_guard = self.shm_time.lock().unwrap();

        time_guard.write(&ts, precision, leap);
    }

    pub fn update_old(&mut self, precision: i32, leap: i32, ts: &Timestamp) {
        let mut time_guard = self.shm_time.lock().unwrap();
        time_guard.write(&ts, precision, leap);
    }

    // NTP reads the shared memory as described at http://doc.ntp.org/4.2.8/drivers/driver28.html
    //
    // In mode 1 it resets valid and bumps count after reading values.  We can't trust valid as it
    // may change while we're reading values.
    //
    // Instead we make a best-effort by tracking count.  If it is different than last go-around and
    // did not change while reading we probably got new values, so we report them.
    pub async fn watch<F>(&self, f: F)
    where
        F: Fn(&crate::shm::Timestamp),
    {
        let time = self.shm_time.lock().unwrap();
        let mut last_count: i32 = 0;

        let mut interval = interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        // TODO use tokio::time::interval
        loop {
            if let Some(timestamp) = time.read(last_count) {
                last_count = timestamp.count;

                f(&timestamp);
            }

            interval.tick().await;
        }
    }
}
