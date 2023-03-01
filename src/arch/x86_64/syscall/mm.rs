use alloc::sync::Arc;
use spin::Mutex;

use crate::scheduler::proc::Process;

pub fn sys_mmap(_proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    todo!()
}