use crate::gpsd::Device;
use serde::Serialize;
use std::convert::From;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename = "DEVICES", tag = "class")]
pub struct Devices {
    devices: Vec<Device>,
    remote: Option<String>,
}

impl From<&crate::devices::Devices> for Devices {
    fn from(devices: &crate::devices::Devices) -> Self {
        let mut gpsd_devices = Vec::new();

        for gps in devices.gps_devices() {
            gpsd_devices.push(gps.into());
        }

        Devices {
            devices: gpsd_devices,
            remote: None,
        }
    }
}
