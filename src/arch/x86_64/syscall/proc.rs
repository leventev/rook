use core::ffi::{c_char, CStr};

use alloc::{slice, string::String, sync::Arc, vec::Vec};
use bitflags::bitflags;
use spin::Mutex;

use crate::{posix::Timeval, scheduler::proc::Process, syscalls};

use super::utils;

bitflags! {
    pub struct CloneFlags: u64 {
        const CLONE_FILES = 1 << 0;
        const CLONE_VM = 1 << 1;
        const CLONE_VFORK = 1 << 2;
    }
}

pub struct CloneArgs {
    pub flags: u64,
    pub pidfd: u64,

    pub child_tid: u64,

    pub parent_tid: u64,

    pub exit_signal: u64,

    pub stack: u64,
    pub stack_size: u64,
    pub tls: u64,
    pub set_tid: u64,

    pub set_tid_size: u64,

    pub cgroup: u64,
}

pub fn sys_getpid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().pid as u64
}

pub fn sys_getppid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().ppid as u64
}

pub fn sys_getuid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().uid as u64
}

pub fn sys_geteuid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().euid as u64
}

pub fn sys_getgid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().gid as u64
}

pub fn sys_getegid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().egid as u64
}

pub fn sys_getcwd(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let len = args[1] as usize;
    let buff = unsafe { slice::from_raw_parts_mut(args[0] as *mut u8, len) };

    match syscalls::proc::getcwd::getcwd(proc, buff) {
        Ok(_) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_getpgid(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let pid = args[0] as isize;

    match syscalls::proc::getpgid::getpgid(proc, pid) {
        Ok(gpid) => gpid as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_setpgid(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let pid = args[0] as usize;
    let pgid = args[1] as usize;

    match syscalls::proc::setpgid::setpgid(proc, pid, pgid) {
        Ok(_) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_chdir(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let path = args[0] as *const u8;
    let path_len = args[1] as usize;

    let path = utils::get_userspace_string(path, path_len);

    match syscalls::proc::chdir::chdir(proc, &path) {
        Ok(n) => n as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_clone(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let clone_args = args[0] as *const CloneArgs;
    let size = args[1] as usize;

    match syscalls::proc::clone::clone(proc, clone_args, size) {
        Ok(pid) => pid as u64,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_execve(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let path = args[0] as *const u8;
    let path_len = args[1] as usize;
    let argv = args[2] as *const *const c_char;
    let envp = args[3] as *const *const c_char;

    let path = utils::get_userspace_string(path, path_len);

    let argv = unsafe { parse_c_char_array(argv) };
    let envp = unsafe { parse_c_char_array(envp) };

    match syscalls::proc::execve::execve(proc, &path, &argv, &envp) {
        Ok(_) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

unsafe fn parse_c_char_array(arr: *const *const c_char) -> Vec<String> {
    let mut vec = Vec::new();

    // TODO: error handling
    // TODO: work with bytes instead of strings

    let mut ptr = arr;
    let mut c_str = *ptr;
    while !c_str.is_null() {
        let str = CStr::from_ptr(c_str).to_str().unwrap();
        let str = String::from(str);
        vec.push(str);

        ptr = ptr.add(1);
        c_str = *ptr;
    }

    vec
}

pub fn sys_archctl(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let req = args[0] as usize;
    let arg = args[1] as usize;

    match syscalls::proc::archctl::archctl(proc, req, arg) {
        Ok(_) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}

pub fn sys_gettimeofday(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    // TODO: validate ptr
    let tv = unsafe { (args[0] as *mut Timeval).as_mut().unwrap() };

    match syscalls::proc::gettimeofday::gettimeofday(proc, tv) {
        Ok(_) => 0,
        Err(err) => err.into_inner_result() as u64,
    }
}
