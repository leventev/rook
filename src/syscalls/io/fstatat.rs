use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    fs::{errors::FsStatError, VFS},
    posix::{errno::Errno, Stat},
    scheduler::proc::Process,
};

pub fn fstatat(
    proc: Arc<Mutex<Process>>,
    fd: isize,
    path: &str,
    stat_buf: *mut Stat,
    _flag: usize,
) -> Result<usize, Errno> {
    // TODO: flag
    let p = proc.lock();

    let full_path = match p.get_full_path_from_dirfd(fd, path) {
        Ok(path) => path,
        Err(_) => todo!(),
    };

    // TODO: validate struct
    let stat_buf = unsafe { stat_buf.as_mut() }.unwrap();

    let mut vfs = VFS.write();
    match vfs.stat(&full_path, stat_buf) {
        Ok(_) => Ok(0),
        Err(err) => match err {
            FsStatError::BadPath(path) => Err(path.into()),
        },
    }
}
