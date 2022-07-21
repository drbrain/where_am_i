use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum GpsType {
    Generic,
    #[serde(rename = "mkt")]
    MKT,
    #[serde(rename = "ublox_nmea")]
    UBloxNMEA,
}

impl Default for GpsType {
    fn default() -> Self {
        GpsType::Generic
    }
}
