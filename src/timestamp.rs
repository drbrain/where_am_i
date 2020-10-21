use crate::pps::ioctl;

use serde_json::json;
use serde_json::Value;

use std::time::Duration;

#[derive(Clone, Debug)]
pub enum TimestampKind {
    GPS,
    PPS,
}

#[derive(Clone, Debug)]
pub struct Timestamp {
    pub device: String,
    pub kind: TimestampKind,
    pub precision: i32,
    pub leap: i32,
    pub real_sec: i64,
    pub real_nsec: i32,
    pub clock_sec: u64,
    pub clock_nsec: u32,
}

impl Timestamp {
    pub fn from_pps_time(
        device: String,
        precision: i32,
        pps_time: ioctl::data,
        now: Duration,
    ) -> Self {
        Timestamp {
            device: device,
            kind: TimestampKind::PPS,
            precision: precision,
            leap: 0,
            real_sec: pps_time.info.assert_tu.sec,
            real_nsec: pps_time.info.assert_tu.nsec,
            clock_sec: now.as_secs(),
            clock_nsec: now.subsec_nanos(),
        }
    }
}

impl Into<Value> for Timestamp {
    fn into(self) -> Value {
        match self.kind {
            TimestampKind::GPS => from_gps(self),
            TimestampKind::PPS => from_pps(self),
        }
    }
}

fn from_gps(t: Timestamp) -> Value {
    json!({
        "class":      "GPS".to_string(),
        "device":     t.device,
        "real_sec":   t.real_sec,
        "real_nsec":  t.real_nsec,
        "clock_sec":  t.clock_sec,
        "clock_nsec": t.clock_nsec,
        "precision":  t.precision,
    })
}

fn from_pps(t: Timestamp) -> Value {
    json!({
        "class":      "PPS".to_string(),
        "device":     t.device,
        "real_sec":   t.real_sec,
        "real_nsec":  t.real_nsec,
        "clock_sec":  t.clock_sec,
        "clock_nsec": t.clock_nsec,
        "precision":  t.precision,
    })
}
