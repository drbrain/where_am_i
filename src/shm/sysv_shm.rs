use libc;

use std::io;
use std::mem;
use std::ptr;

#[derive(Debug, Default)]
#[repr(C)]
pub struct time {
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
    _dummy: [u8; 8],
}

const NTPD_BASE: i32 = 0x4e545030;

pub fn get_id(unit: i32, perms: i32) -> io::Result<i32> {
    let key = NTPD_BASE + unit;
    let size = mem::size_of::<time>();
    let flags = libc::IPC_CREAT | perms;

    let id;

    unsafe {
        id = libc::shmget(key, size, flags);
    }

    if -1 == id {
        Err(io::Error::last_os_error())
    } else {
        Ok(id)
    }
}

pub fn map(id: i32) -> io::Result<Box<time>> {
    let ptr;

    unsafe {
        ptr = libc::shmat(id, ptr::null(), 0);
    }

    if -1 == ptr as i32 {
        Err(io::Error::last_os_error())
    } else {
        let box_time;

        unsafe {
            box_time = Box::from_raw(ptr as *mut time);
        }

        Ok(box_time)
    }
}

pub fn unmap(time: Box<time>) -> io::Result<()> {
    let ptr = Box::into_raw(time) as *const libc::c_void;
    let ok;

    unsafe {
        ok = libc::shmdt(ptr);
    }

    if -1 == ok {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_id() {
        assert_eq!(65538, get_id(2, 0o0600).unwrap());
    }

    #[test]
    fn test_map_unmap() {
        let id = get_id(2, 0o600).unwrap();
        let time = map(id).unwrap();

        assert_eq!(0, time.mode);

        assert_eq!((), unmap(time).unwrap());
    }
}
