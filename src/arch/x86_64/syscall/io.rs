use core::ffi::{c_char, CStr};

use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{
    fs::{
        errors::{FsOpenError, FsStatError},
        SeekWhence, VFS,
    },
    posix::{
        errno::{Errno, EBADF},
        FileOpenFlags, FileOpenMode, Stat, F_DUPFD, F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_SETFD,
        F_SETFL,
    },
    scheduler::proc::Process,
};

pub fn sys_write(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts(args[1] as *const u8, len) };

    match write(proc, fd, buff) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn write(proc: Arc<Mutex<Process>>, fd: usize, buff: &[u8]) -> Result<usize, Errno> {
    let p = proc.lock();
    let file_lock = p.get_fd(fd).ok_or(EBADF)?;

    let mut file_desc = file_lock.lock();
    file_desc.write(buff).map_err(|_| todo!())
}

pub fn sys_read(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts_mut(args[1] as *mut u8, len) };

    match read(proc, fd, buff) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn read(proc: Arc<Mutex<Process>>, fd: usize, buff: &mut [u8]) -> Result<usize, Errno> {
    let p = proc.lock();
    let file_lock = p.get_fd(fd).ok_or(EBADF)?;

    let mut file_desc = file_lock.lock();
    file_desc.read(buff).map_err(|_| todo!())
}

pub fn sys_openat(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let dirfd = args[0] as isize;
    let pathname = args[1] as *const c_char;
    let flags = FileOpenFlags::from_bits_truncate(args[2] as u32);
    let mode = FileOpenMode::from_bits_truncate(args[3] as u32);

    match openat(proc, dirfd, pathname, flags, mode) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn openat(
    proc: Arc<Mutex<Process>>,
    dirfd: isize,
    pathname: *const c_char,
    flags: FileOpenFlags,
    _mode: FileOpenMode,
) -> Result<usize, Errno> {
    // TODO: flags, mode
    let mut p = proc.lock();

    // TODO: validate path
    let path = unsafe { CStr::from_ptr(pathname) }.to_str().unwrap();
    debug!("openat {} {}", dirfd, path);
    let full_path = match p.get_full_path_from_dirfd(dirfd, path) {
        Ok(path) => path,
        Err(_) => todo!(),
    };

    let file_desc = {
        let mut vfs = VFS.write();
        let desc = vfs
            .open(full_path.as_str(), flags)
            .map_err(|err| match err {
                FsOpenError::BadPath(path) => path.into_errno(),
            })?;
        Arc::new(Mutex::new(*desc))
    };

    let fd = p.new_fd(None, file_desc).unwrap();

    Ok(fd)
}

pub fn sys_close(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    match close(proc, fd) {
        Ok(()) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn close(proc: Arc<Mutex<Process>>, fd: usize) -> Result<(), Errno> {
    let mut p = proc.lock();

    if p.get_fd(fd).is_none() {
        return Err(EBADF);
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
        Ok(ret) => ret as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn fstatat(
    proc: Arc<Mutex<Process>>,
    fd: isize,
    path: *const c_char,
    stat_buf: *mut Stat,
    _flag: usize,
) -> Result<usize, Errno> {
    // TODO: flag
    let p = proc.lock();

    // TODO: validate path
    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap();

    let full_path = match p.get_full_path_from_dirfd(fd, path) {
        Ok(path) => path,
        Err(_) => todo!(),
    };

    // TODO: validate struct
    let stat_buf = unsafe { stat_buf.as_mut() }.unwrap();

    let mut vfs = VFS.write();
    match vfs.stat(&full_path, stat_buf) {
        Ok(_) => Ok(0),
        Err(err) => match err {
            FsStatError::BadPath(path) => Err(path.into_errno()),
        },
    }
}

pub fn sys_fcntl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let cmd = args[1] as usize;
    let arg = args[2] as usize;

    match fcntl(proc, fd, cmd, arg) {
        Ok(ret) => ret as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn fcntl(proc: Arc<Mutex<Process>>, fd: usize, cmd: usize, arg: usize) -> Result<usize, Errno> {
    let mut p = proc.lock();

    let node = p.get_fd(fd).ok_or(EBADF)?;

    match cmd {
        F_DUPFD => p.dup_fd(Some(arg), fd).or(Err(EBADF)),
        F_DUPFD_CLOEXEC => {
            warn!("F_DUPFD_CLOEXEC cloexec ignored, doing F_DUPFD instead");
            p.dup_fd(Some(arg), fd).or(Err(EBADF))
        }
        F_GETFD => {
            warn!("fcntl F_GETFD not implemented");
            Ok(0)
        }
        F_SETFD => {
            // TODO
            warn!("fcntl F_SETFD not implemented");
            Ok(0)
        }
        F_GETFL => {
            // TODO: mode
            let flags = node.lock().flags;
            Ok(flags.bits() as usize)
        }
        F_SETFL => {
            let flags = FileOpenFlags::from_bits_truncate(arg as u32);
            node.lock().flags = flags;
            Ok(0)
        }
        _ => todo!(),
    }
}

pub fn sys_ioctl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let req = args[1] as usize;
    let arg = args[2] as usize;

    match ioctl(proc, fd, req, arg) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn ioctl(proc: Arc<Mutex<Process>>, fd: usize, req: usize, arg: usize) -> Result<usize, Errno> {
    let p = proc.lock();

    let file_lock = p.get_fd(fd).ok_or(EBADF)?;

    let file_desc = file_lock.lock();
    match file_desc.ioctl(req, arg) {
        Ok(ret) => Ok(ret),
        Err(_) => todo!(),
    }
}

pub fn sys_lseek(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let offset = args[1] as usize;
    let whence = args[2] as usize;

    match lseek(proc, fd, offset, whence) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn lseek(
    proc: Arc<Mutex<Process>>,
    fd: usize,
    offset: usize,
    whence: usize,
) -> Result<usize, Errno> {
    let p = proc.lock();

    let file_lock = p.get_fd(fd).ok_or(EBADF)?;

    let whence = match whence {
        0 => SeekWhence::Set,
        1 => SeekWhence::Cur,
        2 => SeekWhence::End,
        _ => todo!(),
    };

    let mut file_desc = file_lock.lock();
    match file_desc.lseek(offset, whence) {
        Ok(ret) => Ok(ret),
        Err(_) => todo!(),
    }
}

pub fn sys_chdir(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let path = args[0] as *const c_char;

    match chdir(proc, path) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn chdir(proc: Arc<Mutex<Process>>, path: *const c_char) -> Result<usize, Errno> {
    let mut p = proc.lock();

    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap();
    // TODO: proper flags
    let mut vfs = VFS.write();
    let new_cwd = Arc::new(Mutex::new(match vfs.open(path, FileOpenFlags::empty()) {
        Ok(fd) => *fd,
        Err(_) => todo!(),
    }));

    p.change_cwd(new_cwd);

    Ok(0)
}

pub fn sys_log(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let message = args[0] as *const c_char;

    log(proc, message).unwrap();

    0
}

fn log(proc: Arc<Mutex<Process>>, message: *const c_char) -> Result<(), Errno> {
    // TODO: check if message is valid
    let message = unsafe { CStr::from_ptr(message) }.to_str().unwrap();

    let p = proc.lock();
    log!("process {}: {}", p.pid, message);

    Ok(())
}

pub fn sys_pselect(_proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    1
}
