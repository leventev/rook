use alloc::{sync::Arc, slice};
use spin::Mutex;

use crate::scheduler::proc::Process;

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
        Err(_) => 0
    }
}

pub fn getcwd(proc: Arc<Mutex<Process>>, buff: &mut [u8]) -> Result<(), ()> {
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
