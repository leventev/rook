use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    fs::VFS,
    posix::{errno::Errno, FileOpenFlags},
    scheduler::proc::Process,
};

pub fn chdir(proc: Arc<Mutex<Process>>, path: &str) -> Result<usize, Errno> {
    let mut p = proc.lock();

    // TODO: proper flags
    let mut vfs = VFS.write();
    let new_cwd = Arc::new(Mutex::new(match vfs.open(path, FileOpenFlags::empty()) {
        Ok(fd) => *fd,
        Err(_) => todo!(),
    }));

    p.change_cwd(new_cwd);

    Ok(0)
}
