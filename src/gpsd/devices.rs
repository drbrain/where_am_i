use crate::configuration::GpsConfig;
use crate::gpsd::Device;

use std::convert::From;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Devices {
    pub class: String,
    pub devices: Vec<Device>,
    pub remote: Option<String>,
}

impl From<Vec<GpsConfig>> for Devices {
    fn from(configs: Vec<GpsConfig>) -> Self {
        let mut devices = Vec::with_capacity(configs.len());

        for config in configs.iter() {
            devices.push(config.into());
        }

        Devices {
            class: "DEVICES".to_string(),
            devices,
            remote: None,
        }
    }
}
