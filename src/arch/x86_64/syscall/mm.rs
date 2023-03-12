use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno,
    scheduler::proc::{MappedRegionFlags, Process},
};

#[derive(Debug, Clone, Copy)]
enum SyscallMapError {
    InvalidValue,
}

impl SyscallMapError {
    fn as_errno(&self) -> u64 {
        let val = match self {
            SyscallMapError::InvalidValue => errno::EINVAL,
        };

        (-val) as u64
    }
}

pub fn sys_mmap(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let addr = args[0] as usize;
    let len = args[1] as usize;
    let prot = args[2] as u32;
    let flags = args[3] as u32;
    let fd = args[4] as usize;
    let off = args[5] as u64;

    match mmap(proc, addr, len, prot, flags, fd, off) {
        Ok(n) => n,
        Err(err) => err.as_errno(),
    }
}

fn mmap(
    proc: Arc<Mutex<Process>>,
    addr: usize,
    len: usize,
    prot: u32,
    flags: u32,
    fd: usize,
    off: u64,
) -> Result<u64, SyscallMapError> {
    if prot != 0 || flags != 0 || fd != 0 || off != 0 {
        todo!()
    }

    if addr % 4096 != 0 || len % 4096 != 0 || len == 0 {
        return Err(SyscallMapError::InvalidValue);
    }

    let pages = len / 4096;
    let flags = MappedRegionFlags::READ_WRITE | MappedRegionFlags::ALLOC_ON_ACCESS;

    // TODO: turn flags into MappedRegionFlags
    let mut p = proc.lock();
    match p.add_region(addr, pages, flags) {
        Ok(_) => Ok(addr as u64),
        Err(_) => Err(SyscallMapError::InvalidValue),
    }
}
