use crate::timestamp::Timestamp;

use libc::c_int;

use std::task::Waker;

#[derive(Debug)]
pub struct State {
    pub device: String,
    pub precision: i32,
    pub fd: c_int,
    pub result: Option<Timestamp>,
    pub waker: Option<Waker>,
}

impl State {
    pub fn new(device: String, precision: i32, fd: c_int) -> Self {
        State {
            device,
            precision,
            fd,
            result: None,
            waker: None,
        }
    }
}
