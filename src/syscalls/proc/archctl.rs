use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    mm::VirtAddr,
    posix::errno::Errno,
    scheduler::{proc::Process, thread::ThreadInner},
};

pub fn archctl(proc: Arc<Mutex<Process>>, req: usize, arg: usize) -> Result<(), Errno> {
    const SET_FS: usize = 0x1000;

    let p = proc.lock();

    let main_thread_lock = p.main_thread.upgrade().unwrap();
    let mut main_thread = main_thread_lock.lock();

    // TODO
    match req {
        SET_FS => {
            // TODO: check if fs is valid
            if let ThreadInner::User(data) = &mut main_thread.inner {
                data.tls = VirtAddr::new(arg as u64);
            }
            Ok(())
        }
        _ => todo!(),
    }
}
