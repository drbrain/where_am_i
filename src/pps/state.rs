use crate::timestamp::Timestamp;

use libc::c_int;

#[derive(Debug)]
pub struct State {
    pub device: String,
    pub precision: i32,
    pub fd: c_int,
    pub result: Option<Timestamp>,
}

impl State {
    pub fn new(device: String, precision: i32, fd: c_int) -> Self {
        State {
            device,
            precision,
            fd,
            result: None,
        }
    }
}
