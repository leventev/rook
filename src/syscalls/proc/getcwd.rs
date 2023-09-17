use alloc::sync::Arc;
use spin::Mutex;

use crate::{posix::errno::Errno, scheduler::proc::Process};

pub fn getcwd(proc: Arc<Mutex<Process>>, buff: &mut [u8]) -> Result<(), Errno> {
    let p = proc.lock();
    let cwd = &p.cwd.lock();

    let vnode = cwd.vnode.upgrade().unwrap();
    let vnode = vnode.lock();
    let vnode_path = vnode.get_path();

    if vnode_path.len() > buff.len() {
        todo!()
    }

    let buff = &mut buff[..vnode_path.len() + 1];
    buff[..vnode_path.len()].copy_from_slice(vnode_path.as_bytes());
    buff[buff.len() - 1] = b'\0';

    Ok(())
}
