use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::Errno,
    scheduler::proc::{MappedRegionFlags, Process},
};

pub fn sys_mmap(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let addr = args[0] as usize;
    let len = args[1] as usize;
    let prot = args[2] as u32;
    let flags = args[3] as u32;
    let fd = args[4] as isize;
    let off = args[5] as u64;

    match mmap(proc, addr, len, prot, flags, fd, off) {
        Ok(n) => n,
        Err(err) => err.into_inner_result() as u64,
    }
}

fn mmap(
    proc: Arc<Mutex<Process>>,
    hint: usize,
    len: usize,
    prot: u32,
    flags: u32,
    fd: isize,
    off: u64,
) -> Result<u64, Errno> {
    if prot != 0 || flags != 0 || fd >= 0 || off != 0 {
        todo!()
    }

    let hint = match hint {
        0 => None,
        addr if addr % 4096 == 0 => Some(addr),
        _ => todo!(),
    };

    if len == 0 {
        todo!()
    }

    let flags = MappedRegionFlags::READ_WRITE | MappedRegionFlags::ALLOC_ON_ACCESS;

    // TODO: turn flags into MappedRegionFlags
    let mut p = proc.lock();
    match p.mmap(hint, len, flags) {
        Ok(addr) => Ok(addr as u64),
        Err(_) => todo!(),
    }
}
