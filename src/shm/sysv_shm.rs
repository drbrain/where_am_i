use std::io;
use std::mem;
use std::mem::ManuallyDrop;
use std::ptr;

use volatile::Volatile;

pub type ShmTime = ManuallyDrop<Box<time>>;

#[derive(Debug, Default)]
#[repr(C)]
pub struct time {
    pub mode: i32,
    pub count: Volatile<i32>,
    pub clock_sec: i32,
    pub clock_usec: i32,
    pub receive_sec: i32,
    pub receive_usec: i32,
    pub leap: i32,
    pub precision: i32,
    pub nsamples: i32,
    pub valid: Volatile<i32>,
    pub clock_nsec: u32,
    pub receive_nsec: u32,
    _dummy: [u8; 8],
}

pub fn get_id(key: i32, perms: i32) -> io::Result<i32> {
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

pub fn map(id: i32) -> io::Result<ShmTime> {
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

        Ok(ManuallyDrop::new(box_time))
    }
}

pub fn unmap(time: ShmTime) {
    let time = ManuallyDrop::into_inner(time);
    let ok;

    unsafe {
        let ptr: *mut time = Box::into_raw(time);

        ok = libc::shmdt(ptr as *const libc::c_void);
    }

    if -1 == ok {
        let error = io::Error::last_os_error();
        panic!("unable to unmap shared memory ({:?})", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_id() {
        assert_eq!(65538, get_id(0x4e545032, 0o0666).unwrap());
    }

    #[test]
    fn test_map_unmap() {
        let id = get_id(2, 0o600).unwrap();
        let time = map(id).unwrap();

        assert_eq!(0, time.mode);

        assert_eq!((), unmap(time));
    }
}
