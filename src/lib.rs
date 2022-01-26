pub mod configuration;
pub mod device;
pub mod gps;
pub mod gpsd;
pub mod nmea;
pub mod pps;
pub mod precision;
pub mod shm;
pub mod timestamp;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate nix;

#[cfg(test)]
#[macro_use]
extern crate assert_approx_eq;

use timestamp::Timestamp;
use tokio::sync::broadcast;

pub type TSReceiver = broadcast::Receiver<Timestamp>;
pub type TSSender = broadcast::Sender<Timestamp>;
