use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "WATCH", tag = "class")]
pub struct Watch {
    pub enable: Option<bool>,
    pub json: Option<bool>,
    pub nmea: Option<bool>,
    pub raw: Option<u64>,
    pub scaled: Option<bool>,
    pub split24: Option<bool>,
    pub pps: Option<bool>,
    pub device: Option<String>,
    pub remote: Option<String>,
}

impl Watch {
    pub fn update(&mut self, updates: Watch) {
        if updates.enable.is_some() {
            self.enable = updates.enable;
        }

        if updates.json.is_some() {
            self.json = updates.json;
        }

        if updates.nmea.is_some() {
            self.nmea = updates.nmea;
        }

        if updates.raw.is_some() {
            self.raw = updates.raw;
        }

        if updates.scaled.is_some() {
            self.scaled = updates.scaled;
        }

        if updates.split24.is_some() {
            self.split24 = updates.split24;
        }

        if updates.pps.is_some() {
            self.pps = updates.pps;
        }

        if updates.device.is_some() {
            self.device = updates.device;
        }

        if updates.remote.is_some() {
            self.remote = updates.remote;
        }
    }
}
