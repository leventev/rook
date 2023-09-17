use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::{Errno, EBADF},
    scheduler::proc::Process,
};

pub fn ioctl(proc: Arc<Mutex<Process>>, fd: usize, req: usize, arg: usize) -> Result<usize, Errno> {
    let p = proc.lock();

    let file_lock = p.get_fd(fd).ok_or(EBADF)?;

    let file_desc = file_lock.lock();
    match file_desc.ioctl(req, arg) {
        Ok(ret) => Ok(ret),
        Err(_) => todo!(),
    }
}
