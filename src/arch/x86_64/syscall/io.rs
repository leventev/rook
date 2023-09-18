use core::str::{from_utf8_unchecked, from_utf8_unchecked_mut};

use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{
    posix::{FileOpenFlags, FileOpenMode, Stat},
    scheduler::proc::Process,
    syscalls::{self},
};

use super::utils;

pub fn sys_write(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts(args[1] as *const u8, len) };

    match syscalls::io::write::write(proc, fd, buff) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_read(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let len = args[2] as usize;
    let buff = unsafe { slice::from_raw_parts_mut(args[1] as *mut u8, len) };

    match syscalls::io::read::read(proc, fd, buff) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_openat(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let dirfd = args[0] as isize;

    let path = args[1] as *const u8;
    let path_length = args[2] as usize;

    let flags = FileOpenFlags::from_bits_truncate(args[3] as u32);
    let mode = FileOpenMode::from_bits_truncate(args[4] as u32);

    // TODO: copy path to kernelspace
    let path = utils::get_userspace_string(path, path_length).unwrap();

    match syscalls::io::openat::openat(proc, dirfd, &path, flags, mode) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_close(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    match syscalls::io::close::close(proc, fd) {
        Ok(()) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_fstatat(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as isize;
    let path = args[1] as *const u8;
    let path_len = args[2] as usize;
    // TODO: validate buff
    let stat_buf = unsafe { (args[3] as *mut Stat).as_mut() }.unwrap();
    let flag = args[4] as usize;

    let path = utils::get_userspace_string(path, path_len);

    match syscalls::io::fstatat::fstatat(proc, fd, path.as_deref(), stat_buf, flag) {
        Ok(_) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_fcntl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let cmd = args[1] as usize;
    let arg = args[2] as usize;

    match syscalls::io::fcntl::fcntl(proc, fd, cmd, arg) {
        Ok(ret) => ret as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_ioctl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let req = args[1] as usize;
    let arg = args[2] as usize;

    match syscalls::io::ioctl::ioctl(proc, fd, req, arg) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_lseek(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let offset = args[1] as usize;
    let whence = args[2] as usize;

    match syscalls::io::lseek::lseek(proc, fd, offset, whence) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_log(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let message = args[0] as *const u8;
    let message_len = args[1] as usize;

    // TODO: error
    let message = utils::get_userspace_string(message, message_len).unwrap();

    syscalls::io::log::log(proc, &message).unwrap();

    0
}

pub fn sys_pselect(_proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    1
}

pub fn sys_fd2path(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let fd = args[0] as usize;
    let ptr = args[1] as *mut u8;
    let len = args[2] as usize;

    let buff = unsafe { slice::from_raw_parts_mut(ptr, len) };

    match syscalls::io::fd2path::fd2path(proc, fd, buff) {
        Ok(val) => val as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}
