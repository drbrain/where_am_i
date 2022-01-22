use crate::timestamp::Timestamp;

use libc::c_int;

#[derive(Debug)]
pub struct State {
    pub device: String,
    pub fd: c_int,
    pub result: Option<Timestamp>,
}

impl State {
    pub fn new(device: String, fd: c_int) -> Self {
        State {
            device,
            fd,
            result: None,
        }
    }
}
