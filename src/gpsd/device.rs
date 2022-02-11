use crate::gps::GPS;
use serde::Deserialize;
use serde::Serialize;
use std::convert::From;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "DEVICE", tag = "class")]
pub struct Device {
    pub path: Option<String>,
    pub native: Option<u64>,
}

impl From<&GPS> for Device {
    fn from(gps: &GPS) -> Self {
        Device {
            path: Some(gps.name.clone()),
            native: Some(0),
        }
    }
}
