use alloc::sync::Arc;
use spin::Mutex;

use crate::{scheduler::proc::Process, syscalls};

pub fn sys_mmap(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let addr = args[0] as usize;
    let len = args[1] as usize;
    let prot = args[2] as u32;
    let flags = args[3] as u32;
    let fd = args[4] as isize;
    let off = args[5];

    match syscalls::mm::mmap::mmap(proc, addr, len, prot, flags, fd, off) {
        Ok(n) => n,
        Err(err) => err.into_inner_result() as u64,
    }
}
