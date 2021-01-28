mod gps;
mod gps_data;
pub mod mkt;
pub mod ublox;

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

pub use gps::GPS;
pub use gps_data::GPSData;

#[cfg(test)]
mod test;
