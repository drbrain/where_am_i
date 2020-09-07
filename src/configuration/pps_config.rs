use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PpsConfig {
    pub device: String,
    pub ntp_unit: Option<i32>,
}
