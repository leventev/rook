use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    arch::x86_64::syscall::proc::{CloneArgs, CloneFlags},
    posix::errno::Errno,
    scheduler::{
        proc::Process,
        thread::{ThreadID, ThreadInner},
        SCHEDULER,
    },
};

pub fn clone(
    proc: Arc<Mutex<Process>>,
    clone_args: *const CloneArgs,
    _size: usize,
) -> Result<usize, Errno> {
    // TODO: check if sizeof(clone_args) == size???
    // TODO: validate clone_args

    let child_tid: ThreadID;
    let child_pid: usize;
    let block_wait_for_child: bool;

    {
        let clone_args = unsafe { clone_args.as_ref() }.unwrap();
        let p = proc.lock();

        let child = p.clone_proc(clone_args);
        let child = child.lock();
        child_pid = child.pid;

        {
            let thread = child.main_thread.upgrade().unwrap();
            let mut thread = thread.lock();

            child_tid = thread.id;

            if let ThreadInner::User(data) = &mut thread.inner {
                data.user_regs.general.rax = 0;
                data.in_kernelspace = false;
            }
        }

        let clone_flags = CloneFlags::from_bits(clone_args.flags).unwrap();
        block_wait_for_child = clone_flags.contains(CloneFlags::CLONE_VFORK);
    }

    // TODO: disable interrupts?, maybe scheduler interrupt mutex already does that for us
    SCHEDULER.run_thread(child_tid);

    if block_wait_for_child {
        SCHEDULER.block_current_thread();
    }

    Ok(child_pid)
}
