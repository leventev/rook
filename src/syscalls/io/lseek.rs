use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    fs::SeekWhence,
    posix::errno::{Errno, EBADF},
    scheduler::proc::Process,
};

pub fn lseek(
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
