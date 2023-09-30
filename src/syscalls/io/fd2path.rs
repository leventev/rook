use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::{Errno, EBADF, EINVAL},
    scheduler::proc::Process,
};

pub fn fd2path(proc: Arc<Mutex<Process>>, fd: usize, buff: &mut [u8]) -> Result<usize, Errno> {
    let p = proc.lock();

    let file = p.get_fd(fd).ok_or(EBADF)?;

    let file = file.lock();
    let vnode = file.vnode.upgrade().unwrap();
    let vnode = vnode.lock();

    let path = vnode.get_path();

    if buff.len() < path.len() {
        return Err(EINVAL);
    }

    let buff = &mut buff[..path.len()];
    buff.copy_from_slice(path.as_bytes());

    Ok(path.len())
}
