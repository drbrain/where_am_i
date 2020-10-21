use crate::pps::ioctl;
use crate::pps::State;

use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Time {
    pub device: String,
    pub real_sec: i64,
    pub real_nsec: i32,
    pub clock_sec: u64,
    pub clock_nsec: u32,
    pub precision: i32,
}

impl Time {
    pub fn new(state: &State, pps_time: ioctl::data, now: Duration) -> Self {
        Time {
            device: state.device.clone(),
            real_sec: pps_time.info.assert_tu.sec,
            real_nsec: pps_time.info.assert_tu.nsec,
            clock_sec: now.as_secs(),
            clock_nsec: now.subsec_nanos(),
            precision: state.precision,
        }
    }
}
