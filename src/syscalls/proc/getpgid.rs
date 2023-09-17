use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::Errno,
    scheduler::{self, proc::Process},
};

pub fn getpgid(proc: Arc<Mutex<Process>>, pid: isize) -> Result<usize, Errno> {
    if pid < 0 {
        todo!()
    }

    if pid == 0 {
        return Ok(proc.lock().pgid);
    }

    match scheduler::proc::get_process(pid as usize) {
        Some(proc) => Ok(proc.lock().pgid),
        None => todo!(),
    }
}
