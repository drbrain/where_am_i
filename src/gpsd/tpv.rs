use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename = "TPV", tag = "class")]
pub struct Tpv {
    pub device: String,
    pub time: String,
    pub mode: u32,
}
