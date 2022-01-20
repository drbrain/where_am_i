use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename = "TOFF", tag = "class")]
pub struct Toff {
    pub device: String,
    pub real_sec: i64,
    pub real_nsec: u32,
    pub clock_sec: u64,
    pub clock_nsec: u32,
}
