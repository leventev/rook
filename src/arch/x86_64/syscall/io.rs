use core::ffi::{c_char, CStr};

use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{
    fs::{self, FileSystemError, SeekWhence},
    posix::{
        errno, FileOpenFlags, FileOpenMode, Stat, F_DUPFD, F_GETFD, F_GETFL, F_SETFD, F_SETFL,
    },
    scheduler::proc::Process,
};

#[derive(Debug, Clone, Copy)]
enum SyscallIOError {
    InvalidFD,
    InvalidPath,
    InvalidWhence,
}

impl SyscallIOError {
    fn as_errno(&self) -> u64 {
        let val = match self {
            SyscallIOError::InvalidFD => errno::EBADF,
            // TODO: dirname error
            SyscallIOError::InvalidPath | SyscallIOError::InvalidWhence => errno::EINVAL,
        };

        (-val) as u64
    }
}

impl FileSystemError {
    fn as_syscall_io_error(&self) -> SyscallIOError {
        match self {
            FileSystemError::FileNotFound => SyscallIOError::InvalidPath,
            _ => unreachable!(),
        }
    }
}

pub fn sys_write(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts(args[1] as *const u8, len) };

    match write(proc, fd, len, buff) {
        Ok(n) => n as u64,
        Err(err) => err.as_errno(),
    }
}

fn write(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    len: usize,
    buff: &[u8],
) -> Result<usize, SyscallIOError> {
    let p = proc.lock();
    let file_lock = match p.get_fd(fd) {
        Some(f) => f,
        None => return Err(SyscallIOError::InvalidFD),
    };

    let mut file_desc = file_lock.lock();
    match file_desc.write(len, buff) {
        Ok(written) => Ok(written),
        Err(err) => Err(err.as_syscall_io_error()),
    }
}

pub fn sys_read(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts_mut(args[1] as *mut u8, len) };

    match read(proc, fd, len, buff) {
        Ok(n) => n as u64,
        Err(err) => err.as_errno(),
    }
}

fn read(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    len: usize,
    buff: &mut [u8],
) -> Result<usize, SyscallIOError> {
    let p = proc.lock();
    let file_lock = match p.get_fd(fd) {
        Some(f) => f,
        None => return Err(SyscallIOError::InvalidFD),
    };

    let mut file_desc = file_lock.lock();
    match file_desc.read(len, buff) {
        Ok(read) => Ok(read),
        Err(err) => Err(err.as_syscall_io_error()),
    }
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

    let file_desc = {
        let desc = match fs::open(full_path.as_str()) {
            Ok(desc) => desc,
            Err(err) => return Err(err.as_syscall_io_error()),
        };
        Arc::new(Mutex::new(*desc))
    };

    let fd = p.new_fd(None, file_desc).unwrap();

    Ok(fd)
}

pub fn sys_close(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    match close(proc, fd) {
        Ok(()) => 0,
        Err(err) => err.as_errno(),
    }
}

fn close(proc: Arc<Mutex<Process>>, fd: usize) -> Result<(), SyscallIOError> {
    let mut p = proc.lock();

    if p.get_fd(fd).is_none() {
        return Err(SyscallIOError::InvalidFD);
    }

    p.free_fd(fd);

    Ok(())
}

pub fn sys_fstatat(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as isize;
    let path = args[1] as *const c_char;
    let stat_buf = args[2] as *mut Stat;
    let flag = args[3] as usize;

    match fstatat(proc, fd, path, stat_buf, flag) {
        Ok(ret) => ret,
        Err(err) => err.as_errno(),
    }
}

fn fstatat(
    proc: Arc<Mutex<Process>>,
    fd: isize,
    path: *const c_char,
    stat_buf: *mut Stat,
    _flag: usize,
) -> Result<u64, SyscallIOError> {
    // TODO: flag
    let p = proc.lock();

    // TODO: validate path
    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap();

    let full_path = match p.get_full_path_from_dirfd(fd, path) {
        Ok(path) => path,
        Err(_) => return Err(SyscallIOError::InvalidFD),
    };

    // TODO: validate struct
    let stat_buf = unsafe { stat_buf.as_mut() }.unwrap();

    match fs::stat(&full_path, stat_buf) {
        Ok(_) => Ok(0),
        Err(err) => Err(err.as_syscall_io_error()),
    }
}

pub fn sys_fcntl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let cmd = args[1] as usize;
    let arg = args[2] as usize;

    match fcntl(proc, fd, cmd, arg) {
        Ok(ret) => ret as u64,
        Err(err) => err.as_errno(),
    }
}

fn fcntl(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    cmd: usize,
    arg: usize,
) -> Result<usize, SyscallIOError> {
    let mut p = proc.lock();

    match cmd {
        F_DUPFD => match p.dup_fd(Some(arg), fd) {
            Ok(new_fd) => Ok(new_fd),
            Err(_) => Err(SyscallIOError::InvalidFD),
        },
        F_GETFD => Ok(0),
        F_SETFD => {
            // TODO
            Ok(0)
        }
        F_GETFL => {
            warn!("fcntl F_GETFL not implemented");
            Ok(0)
        }
        F_SETFL => {
            warn!("fcntl F_SETFL not implemented");
            Ok(0)
        }
        _ => unreachable!(),
    }
}

pub fn sys_ioctl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let req = args[1] as usize;
    let arg = args[2] as usize;

    match ioctl(proc, fd, req, arg) {
        Ok(n) => n as u64,
        Err(err) => err.as_errno(),
    }
}

fn ioctl(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    req: usize,
    arg: usize,
) -> Result<usize, SyscallIOError> {
    let p = proc.lock();

    let file_lock = match p.get_fd(fd) {
        Some(f) => f,
        None => return Err(SyscallIOError::InvalidFD),
    };

    let file_desc = file_lock.lock();
    match file_desc.ioctl(req, arg) {
        Ok(ret) => Ok(ret),
        Err(err) => Err(err.as_syscall_io_error()),
    }
}

pub fn sys_lseek(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let offset = args[1] as usize;
    let whence = args[2] as usize;

    match lseek(proc, fd, offset, whence) {
        Ok(n) => n as u64,
        Err(err) => err.as_errno(),
    }
}

fn lseek(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    offset: usize,
    whence: usize,
) -> Result<usize, SyscallIOError> {
    let p = proc.lock();

    let file_lock = match p.get_fd(fd) {
        Some(f) => f,
        None => return Err(SyscallIOError::InvalidFD),
    };

    let whence = match whence {
        0 => SeekWhence::Set,
        1 => SeekWhence::Cur,
        2 => SeekWhence::End,
        _ => return Err(SyscallIOError::InvalidWhence),
    };

    let mut file_desc = file_lock.lock();
    match file_desc.lseek(offset, whence) {
        Ok(ret) => Ok(ret),
        Err(err) => Err(err.as_syscall_io_error()),
    }
}
