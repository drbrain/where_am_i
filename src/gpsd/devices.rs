use crate::gpsd::Device;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Devices {
    pub class: String,
    pub devices: Vec<Device>,
    pub remote: Option<String>,
}
