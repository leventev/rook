use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    fs::{errors::FsOpenError, VFS},
    posix::{errno::Errno, FileOpenFlags, FileOpenMode},
    scheduler::proc::Process,
};

pub fn openat(
    proc: Arc<Mutex<Process>>,
    dirfd: isize,
    path: &str,
    flags: FileOpenFlags,
    _mode: FileOpenMode,
) -> Result<usize, Errno> {
    // TODO: flags, mode
    let mut p = proc.lock();

    // TODO: validate path

    debug!("openat {} {}", dirfd, path);
    let full_path = match p.get_full_path_from_dirfd(dirfd, path) {
        Ok(path) => path,
        Err(_) => todo!(),
    };

    let file_desc = {
        let mut vfs = VFS.write();
        let desc = vfs
            .open(full_path.as_str(), flags)
            .map_err(|err| match err {
                FsOpenError::BadPath(path) => path.into(),
            })?;
        Arc::new(Mutex::new(*desc))
    };

    let fd = p.new_fd(None, file_desc).unwrap();

    Ok(fd)
}
