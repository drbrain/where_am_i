use crate::gps::GPS;
use crate::pps::PPS;

#[derive(Clone, Debug)]
pub enum Device {
    GPS(GPS),
    PPS(PPS),
}
