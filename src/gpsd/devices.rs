use crate::configuration::GpsConfig;
use crate::gpsd::Device;

use std::convert::From;

use serde::Serialize;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename = "DEVICES", tag = "class")]
pub struct Devices {
    devices: Vec<Device>,
    remote: Option<String>,
}

impl From<Vec<GpsConfig>> for Devices {
    fn from(configs: Vec<GpsConfig>) -> Self {
        let mut devices = Vec::with_capacity(configs.len());

        for config in configs.iter() {
            devices.push(config.into());
        }

        Devices {
            devices,
            remote: None,
        }
    }
}
