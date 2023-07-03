use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{
    posix::errno,
    scheduler::{self, proc::{Process, self}},
};

struct CloneArgs {
    flags: u64,
    pidfd: u64,

    child_tid: u64,

    parent_tid: u64,

    exit_signal: u64,

    stack: u64,
    stack_size: u64,
    tls: u64,
    set_tid: u64,

    set_tid_size: u64,

    cgroup: u64,
}

#[derive(Debug, Clone, Copy)]
enum SyscallProcError {
    InvalidPID,
    NotFoundPID,
}

impl SyscallProcError {
    fn as_errno(&self) -> u64 {
        let val = match self {
            Self::InvalidPID => errno::EINVAL,
            Self::NotFoundPID => errno::ESRCH,
        };

        (-val) as u64
    }
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

    match getcwd(proc, buff) {
        Ok(_) => args[0],
        Err(_) => 0,
    }
}

fn getcwd(proc: Arc<Mutex<Process>>, buff: &mut [u8]) -> Result<(), ()> {
    let p = proc.lock();
    let vnode = &p.cwd.lock().vnode;
    let vnode_path = vnode.path();

    if vnode_path.len() > buff.len() {
        return Err(());
    }

    let buff = &mut buff[..vnode_path.len() + 1];
    buff[..vnode_path.len()].copy_from_slice(vnode_path.as_bytes());
    buff[buff.len() - 1] = b'\0';

    Ok(())
}

pub fn sys_getpgid(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let pid = args[0] as isize;

    match getpgid(proc, pid) {
        Ok(gpid) => gpid as u64,
        Err(err) => err.as_errno(),
    }
}

fn getpgid(proc: Arc<Mutex<Process>>, pid: isize) -> Result<usize, SyscallProcError> {
    if pid < 0 {
        return Err(SyscallProcError::InvalidPID);
    }

    if pid == 0 {
        return Ok(proc.lock().pgid);
    }

    match scheduler::proc::get_process(pid as usize) {
        Some(proc) => Ok(proc.lock().pgid),
        None => Err(SyscallProcError::NotFoundPID),
    }
}

pub fn sys_setpgid(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let pid = args[0] as isize;
    let pgid = args[1] as isize;

    match setpgid(proc, pid, pgid) {
        Ok(_) => 0,
        Err(err) => err.as_errno(),
    }
}

fn setpgid(proc: Arc<Mutex<Process>>, pid: isize, pgid: isize) -> Result<(), SyscallProcError> {
    if pid < 0 {
        return Err(SyscallProcError::InvalidPID);
    }

    if pgid < 0 {
        // TODO: new error ?
        return Err(SyscallProcError::InvalidPID);
    }

    assert!(pid == 0, "Only PID == 0 is implemented");

    proc.lock().pgid = pgid as usize;

    Ok(())
}

pub fn sys_clone(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let clone_args = args[0] as *const CloneArgs;
    let size = args[1] as usize;

    match clone(proc, clone_args, size) {
        Ok(pid) => pid as u64,
        Err(err) => err.as_errno(),
    }
}

fn clone(
    proc: Arc<Mutex<Process>>,
    clone_args: *const CloneArgs,
    _size: usize,
) -> Result<usize, SyscallProcError> {
    // TODO: check if sizeof(clone_args) == size???

    //let mut p = proc.lock();

    //let child = Process::new();


    todo!()
}
