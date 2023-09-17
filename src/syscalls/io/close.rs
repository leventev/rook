use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::{Errno, EBADF},
    scheduler::proc::Process,
};

pub fn close(proc: Arc<Mutex<Process>>, fd: usize) -> Result<(), Errno> {
    let mut p = proc.lock();

    if p.get_fd(fd).is_none() {
        return Err(EBADF);
    }

    p.free_fd(fd);

    Ok(())
}
