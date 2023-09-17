use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::{Errno, EBADF},
    scheduler::proc::Process,
};

pub fn read(proc: Arc<Mutex<Process>>, fd: usize, buff: &mut [u8]) -> Result<usize, Errno> {
    let p = proc.lock();
    let file_lock = p.get_fd(fd).ok_or(EBADF)?;

    let mut file_desc = file_lock.lock();
    file_desc.read(buff).map_err(|_| todo!())
}
