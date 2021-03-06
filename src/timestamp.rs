use crate::pps::ioctl;

use serde_json::json;
use serde_json::Value;

use std::time::Duration;

#[derive(Clone, Debug)]
pub enum TimestampKind {
    GPS,
    PPS,
}

/// A timestamp to be sent to (or read from) NTP.
///
/// A timestamp includes both a "real" value and a "clock" value.
///
/// The "clock" value is the time read from a reference clock.  This is the time the other clock
/// thinks the current time is.
///
/// The "real" value is the time of the system or "wall" clock when the timestamp was read.  It may
/// be different than the clock time if the system clock and the reference clock are not
/// synchronized.
#[derive(Clone, Debug)]
pub struct Timestamp {
    /// Device the timestamp was read from
    pub device: String,
    /// Kind of device the timestamp was read from
    pub kind: TimestampKind,
    /// Precision of the timestamp.
    pub precision: i32,
    /// Nonzero if a leap second is coming
    pub leap: i32,
    /// The system clock seconds this timestamp was received
    pub received_sec: u64,
    /// The system clock nanoseconds since the last second boundary this timestamp was received
    pub received_nsec: u32,
    /// The reference clock seconds of this timestamp
    pub reference_sec: u64,
    /// The reference clock nanoseconds since the last second boundary of this timestamp
    pub reference_nsec: u32,
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
            reference_sec: pps_time.info.assert_tu.sec as u64,
            reference_nsec: pps_time.info.assert_tu.nsec as u32,
            received_sec: now.as_secs(),
            received_nsec: now.subsec_nanos(),
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
        "real_sec":   t.reference_sec,
        "real_nsec":  t.reference_nsec,
        "clock_sec":  t.received_sec,
        "clock_nsec": t.received_nsec,
        "precision":  t.precision,
    })
}

fn from_pps(t: Timestamp) -> Value {
    json!({
        "class":      "PPS".to_string(),
        "device":     t.device,
        "real_sec":   t.reference_sec,
        "real_nsec":  t.reference_nsec,
        "clock_sec":  t.received_sec,
        "clock_nsec": t.received_nsec,
        "precision":  t.precision,
    })
}
