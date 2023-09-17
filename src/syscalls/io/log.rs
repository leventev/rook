use alloc::sync::Arc;
use spin::Mutex;

use crate::{posix::errno::Errno, scheduler::proc::Process};

pub fn log(proc: Arc<Mutex<Process>>, message: &str) -> Result<(), Errno> {
    let p = proc.lock();
    log!("process {}: {}", p.pid, message);

    Ok(())
}
