pub mod configuration;
pub mod gps;
pub mod gpsd;
pub mod nmea;
pub mod pps;
pub mod shm;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate nix;

#[cfg(test)]
#[macro_use]
extern crate assert_approx_eq;

use serde_json::Value;
use tokio::sync::broadcast;

pub type JsonReceiver = broadcast::Receiver<Value>;
pub type JsonSender = broadcast::Sender<Value>;
