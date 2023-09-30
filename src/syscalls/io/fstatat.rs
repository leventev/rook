use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    fs::{errors::FsStatError, VFS},
    posix::{
        errno::{Errno, EBADF},
        Stat,
    },
    scheduler::proc::Process,
};

pub fn fstatat(
    proc: Arc<Mutex<Process>>,
    fd: isize,
    path: Option<&str>,
    stat_buf: &mut Stat,
    _flag: usize,
) -> Result<(), Errno> {
    // TODO: flag
    let p = proc.lock();
    if fd < 0 {
        return Err(EBADF);
    };

    let fd = fd as usize;

    match path {
        Some(path) => {
            let full_path = p.get_full_path_from_dirfd(Some(fd), path).unwrap();
            let mut vfs = VFS.write();
            match vfs.stat(&full_path, stat_buf) {
                Ok(_) => Ok(()),
                Err(err) => match err {
                    FsStatError::BadPath(path) => Err(path.into()),
                },
            }
        }
        None => {
            let file_desc = p.get_fd(fd).ok_or(EBADF)?;
            let file_desc = file_desc.lock();
            file_desc.stat(stat_buf).map_err(|err| err.into())
        }
    }
}
