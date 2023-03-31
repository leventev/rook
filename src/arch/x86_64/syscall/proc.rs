use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{
    posix::errno,
    scheduler::{self, proc::Process},
};

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
