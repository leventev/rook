use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::Errno,
    scheduler::proc::{MappedRegionFlags, Process},
};

pub fn mmap(
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
