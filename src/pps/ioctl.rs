use libc::c_int;

use std::mem;

#[derive(Default)]
#[repr(C)]
pub struct data {
    pub info:    info,
    pub timeout: time,
}

#[derive(Default)]
#[repr(C)]
pub struct info {
    pub assert_sequence: u32,   // sequence number of assert event
    pub clear_sequence:  u32,   // sequence number of clear event
    pub assert_tu:       time, // time of assert event
    pub clear_tu:        time, // time of clear event
    pub current_mode:    i32,   // current mode
}

#[derive(Default)]
#[repr(C)]
pub struct params {
    pub api_version:   i32,   // API version
    pub mode:          i32,   // current mode
    pub assert_off_tu: time, // assert offset compensation
    pub clear_off_tu:  time, // clear offset compensation
}

#[derive(Default)]
#[repr(C)]
pub struct time {
    pub sec:   i64, // seconds
    pub nsec:  i32, // nanoseconds
    pub flags: u32, // flags
}

pub const TIME_INVALID: u32 = 1<<0;

pub const CAPTUREASSERT: i32 = 0x01;   // capture assert events
// pub const CAPTURECLEAR:  i32 = 0x02;   // capture clear events
// pub const CAPTUREBOTH:   i32 = 0x03;   // capture both event types

// pub const OFFSETASSERT:  i32 = 0x10;   // apply compensation for assert event
// pub const OFFSETCLEAR:   i32 = 0x20;   // apply compensation for clear event

// pub const ECHOASSERT:    i32 = 0x40;   // feed back assert event to output
// pub const ECHOCLEAR:     i32 = 0x80;   // feed back clear event to output

pub const CANWAIT:       i32 = 0x100;  // Can we wait for an event?
// pub const CANPOLL:       i32 = 0x200;  // Reserved

// pub const DSFMT_TSPEC:   i32 = 0x1000; // struct timespec format
// pub const DSFMT_NTPFP:   i32 = 0x2000; // NTP time format

pub const MAGIC: u8 = b'p';

pub const GETPARAMS: u8 = 0xa1;
pub const SETPARAMS: u8 = 0xa2;
pub const GETCAP:    u8 = 0xa3;
pub const FETCH:     u8 = 0xa4;

// ioctl_read!(getparams, MAGIC, GETPARAMS, params);
pub unsafe fn getparams(fd: c_int, data: *mut params) -> nix::Result<c_int> {
    let res = libc::ioctl(fd, request_code_read!(MAGIC, GETPARAMS, mem::size_of::<*mut params>()), data);
    nix::errno::Errno::result(res)
}

// ioctl_write_ptr!(setparams, MAGIC, SETPARAMS, params);
pub unsafe fn setparams(fd: c_int, data: *mut params) -> nix::Result<c_int> {
    let res = libc::ioctl(fd, request_code_write!(MAGIC, SETPARAMS, mem::size_of::<*mut params>()), data);
    nix::errno::Errno::result(res)
}

ioctl_read!(getcap, MAGIC, GETCAP, i32);

//ioctl_readwrite!(fetch, MAGIC, FETCH, data);
pub unsafe fn fetch(fd: c_int, data: *mut data) -> nix::Result<c_int> {
    let res = libc::ioctl(fd, request_code_readwrite!(MAGIC, FETCH, mem::size_of::<*mut data>()), data);
    nix::errno::Errno::result(res)
}

