#[derive(Clone, Debug)]
pub struct Timestamp {
    pub mode: i32,
    pub count: i32,
    pub clock_sec: i32,
    pub clock_usec: i32,
    pub receive_sec: i32,
    pub receive_usec: i32,
    pub leap: i32,
    pub precision: i32,
    pub nsamples: i32,
    pub valid: i32,
    pub clock_nsec: u32,
    pub receive_nsec: u32,
}
