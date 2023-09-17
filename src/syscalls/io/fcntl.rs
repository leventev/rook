use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    posix::{
        errno::{Errno, EBADF},
        FileOpenFlags, F_DUPFD, F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_SETFD, F_SETFL,
    },
    scheduler::proc::Process,
};

pub fn fcntl(proc: Arc<Mutex<Process>>, fd: usize, cmd: usize, arg: usize) -> Result<usize, Errno> {
    let mut p = proc.lock();

    let node = p.get_fd(fd).ok_or(EBADF)?;

    match cmd {
        F_DUPFD => p.dup_fd(Some(arg), fd).or(Err(EBADF)),
        F_DUPFD_CLOEXEC => {
            warn!("F_DUPFD_CLOEXEC cloexec ignored, doing F_DUPFD instead");
            p.dup_fd(Some(arg), fd).or(Err(EBADF))
        }
        F_GETFD => {
            warn!("fcntl F_GETFD not implemented");
            Ok(0)
        }
        F_SETFD => {
            // TODO
            warn!("fcntl F_SETFD not implemented");
            Ok(0)
        }
        F_GETFL => {
            // TODO: mode
            let flags = node.lock().flags;
            Ok(flags.bits() as usize)
        }
        F_SETFL => {
            let flags = FileOpenFlags::from_bits_truncate(arg as u32);
            node.lock().flags = flags;
            Ok(0)
        }
        _ => todo!(),
    }
}
