use core::ffi::{c_char, CStr};

use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{
    fs,
    posix::{errno, FileOpenFlags, FileOpenMode},
    scheduler::proc::Process,
};

#[derive(Debug, Clone, Copy)]
enum SyscallIOError {
    InvalidFD,
}

impl SyscallIOError {
    fn as_errno(&self) -> u64 {
        let val = match self {
            SyscallIOError::InvalidFD => errno::EBADF,
        };

        (-val) as u64
    }
}

pub fn sys_write(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts(args[1] as *const u8, len) };

    match write(proc, fd, len, buff) {
        Ok(n) => n,
        Err(err) => err.as_errno(),
    }
}

fn write(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    len: usize,
    buff: &[u8],
) -> Result<u64, SyscallIOError> {
    let p = proc.lock();
    let file_lock = match p.get_fd(fd) {
        Some(f) => f,
        None => return Err(SyscallIOError::InvalidFD),
    };

    let mut file_desc = file_lock.lock();
    let written = file_desc.write(len, buff).unwrap();

    Ok(written as u64)
}

pub fn sys_read(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts_mut(args[1] as *mut u8, len) };

    match read(proc, fd, len, buff) {
        Ok(n) => n,
        Err(err) => err.as_errno(),
    }
}

fn read(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    len: usize,
    buff: &mut [u8],
) -> Result<u64, SyscallIOError> {
    let p = proc.lock();
    let file_lock = match p.get_fd(fd) {
        Some(f) => f,
        None => return Err(SyscallIOError::InvalidFD),
    };

    let mut file_desc = file_lock.lock();
    let written = file_desc.read(len, buff).unwrap();

    Ok(written as u64)
}

pub fn sys_openat(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let dirfd = args[0] as isize;
    let pathname = args[1] as *const c_char;
    let flags = FileOpenFlags::from_bits(args[2] as u32).unwrap();
    let mode = FileOpenMode::from_bits(args[3] as u32).unwrap();

    match openat(proc, dirfd, pathname, flags, mode) {
        Ok(n) => n as u64,
        Err(err) => err.as_errno(),
    }
}

fn openat(
    proc: Arc<Mutex<Process>>,
    dirfd: isize,
    pathname: *const c_char,
    _flags: FileOpenFlags,
    _mode: FileOpenMode,
) -> Result<usize, SyscallIOError> {
    // TODO: flags, mode
    let mut p = proc.lock();

    // TODO: validate path
    let path = unsafe { CStr::from_ptr(pathname) }.to_str().unwrap();

    let full_path = match p.get_full_path_from_dirfd(dirfd, path) {
        Ok(path) => path,
        Err(_) => return Err(SyscallIOError::InvalidFD),
    };

    // TODO: invalid path error
    let file_desc = {
        let desc = fs::open(full_path.as_str()).unwrap();
        Arc::new(Mutex::new(*desc))
    };
    let fd = p.new_fd(None, file_desc).unwrap();

    Ok(fd)
}
