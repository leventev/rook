use alloc::sync::Arc;
use spin::Mutex;

use crate::scheduler::proc::Process;

pub fn sys_getpid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().pid as u64
}
