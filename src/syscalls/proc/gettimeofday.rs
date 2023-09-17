use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::{errno::Errno, Timeval},
    scheduler::proc::Process,
    time,
};

pub fn gettimeofday(_proc: Arc<Mutex<Process>>, tv: &mut Timeval) -> Result<(), Errno> {
    let time = time::elapsed();

    tv.tv_sec = time.seconds;
    tv.tv_usec = time.milliseconds;

    Ok(())
}
