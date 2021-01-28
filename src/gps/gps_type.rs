use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GpsType {
    Generic,
    MKT,
    UBlox,
}

impl Default for GpsType {
    fn default() -> Self {
        GpsType::Generic
    }
}
