use crate::pps::Time;

use libc::c_int;

use std::task::Waker;

#[derive(Debug)]
pub struct State {
    pub device: String,
    pub precision: i32,
    pub fd: c_int,
    pub result: Option<Time>,
    pub ok: bool,
    pub completed: bool,
    pub waker: Option<Waker>,
}

impl State {
    pub fn new(device: String, precision: i32, fd: c_int) -> Self {
        State {
            device,
            precision,
            fd,
            result: None,
            ok: false,
            completed: false,
            waker: None,
        }
    }
}
