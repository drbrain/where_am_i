use crate::{gps::GPS, pps::PPS};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub enum Device {
    GPS(GPS),
    PPS(PPS, watch::Receiver<i32>),
}

impl Device {
    pub fn start(&self) {
        match self {
            Device::GPS(gps) => gps.start(),
            Device::PPS(_, _) => (),
        }
    }
}
