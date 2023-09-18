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
    let null_terminated_len = path.len() + 1;

    if buff.len() < null_terminated_len {
        return Err(EINVAL);
    }

    let buff = &mut buff[..null_terminated_len];
    buff[..path.len()].copy_from_slice(path.as_bytes());
    buff[path.len()] = b'\0';

    Ok(null_terminated_len)
}
