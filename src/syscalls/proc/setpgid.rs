use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::errno::Errno,
    scheduler::proc::{get_process, Process},
};

pub fn setpgid(proc: Arc<Mutex<Process>>, pid: usize, pgid: usize) -> Result<(), Errno> {
    // TODO: session leader checks, etc...
    let p = if pid == 0 {
        proc
    } else {
        match get_process(pid) {
            Some(p) => p,
            None => todo!(),
        }
    };

    let mut p = p.lock();
    let new_pgid = if pgid == 0 { p.pid } else { pgid };

    p.pgid = new_pgid;

    Ok(())
}
