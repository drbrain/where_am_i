use super::parser::json_to_string;

use serde::Deserialize;
use serde::Serialize;

use serde_json::Value;

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

impl Watch {
    pub fn update(&mut self, json: Value) {
        if json["enable"].is_boolean() {
            self.enable = json["enable"].as_bool().unwrap_or(false);
        }

        if json["json"].is_boolean() {
            self.json = json["json"].as_bool().unwrap_or(false);
        }

        if json["nmea"].is_boolean() {
            self.nmea = json["nmea"].as_bool().unwrap_or(false);
        }

        if json["raw"].is_u64() {
            self.raw = json["raw"].as_u64().unwrap_or(0);
        }

        if json["scaled"].is_boolean() {
            self.scaled = json["scaled"].as_bool().unwrap_or(false);
        }

        if json["split24"].is_boolean() {
            self.split24 = json["split24"].as_bool().unwrap_or(false);
        }

        if json["pps"].is_boolean() {
            self.pps = json["pps"].as_bool().unwrap_or(false);
        }

        if json["device"].is_string() {
            self.device = json_to_string(&json["device"]);
        }

        if json["remote"].is_string() {
            self.remote = json_to_string(&json["remote"]);
        }
    }
}
