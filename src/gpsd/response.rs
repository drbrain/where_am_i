use crate::gpsd::Device;
use crate::gpsd::Devices;
use crate::gpsd::Toff;
use crate::gpsd::Tpv;
use crate::gpsd::Watch;
use crate::Timestamp;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub enum Response {
    Device(Device),
    Devices(Devices),
    Error(ErrorMessage),
    Poll(Poll),
    Toff(Toff),
    Tpv(Tpv),
    PPS(PPS),
    Version(Version),
    Watch(Watch),
}

impl From<(&String, Timestamp)> for Response {
    fn from(from: (&String, Timestamp)) -> Response {
        let (device, timestamp) = from;

        Response::PPS(PPS {
            device: device.to_string(),
            real_sec: timestamp.reference_sec,
            real_nsec: timestamp.reference_nsec,
            clock_sec: timestamp.received_sec,
            clock_nsec: timestamp.received_nsec,
            precision: timestamp.precision,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename = "ERROR", tag = "class")]
pub struct ErrorMessage {
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename = "POLL", tag = "class")]
pub struct Poll {
    pub time: f64,
    pub active: u32,
    pub tpv: Vec<Tpv>,
    pub sky: Vec<Sky>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "class")]
pub struct PPS {
    pub device: String,
    pub real_sec: u64,
    pub real_nsec: u32,
    pub clock_sec: u64,
    pub clock_nsec: u32,
    pub precision: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename = "SKY", tag = "class")]
pub struct Sky {}

#[derive(Clone, Debug, Serialize)]
#[serde(rename = "VERSION", tag = "class")]
pub struct Version {
    pub release: String,
    pub rev: String,
    pub proto_major: u32,
    pub proto_minor: u32,
}
