use crate::{gps::GPS, pps::PPS};
use lazy_static::lazy_static;
use prometheus::{register_int_counter_vec, IntCounterVec};
use tokio::sync::watch;

lazy_static! {
    pub(crate) static ref DEVICE_OPENS: IntCounterVec = register_int_counter_vec!(
        "where_am_i_device_opens_count",
        "Count of times a device was opened with result",
        &["device", "result"]
    )
    .unwrap();
}

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
