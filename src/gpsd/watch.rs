use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Watch {
    pub class: String,
    pub enable: bool,
    pub json: bool,
    pub nmea: bool,
    pub raw: u64,
    pub scaled: bool,
    pub split24: bool,
    pub pps: bool,
    pub device: Option<String>,
    pub remote: Option<String>,
}

