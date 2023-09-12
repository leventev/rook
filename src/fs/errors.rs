use crate::posix::errno::{Errno, ENOENT, ENOTDIR, EACCES};

use super::path::PathParseError;

#[derive(Debug)]
pub enum FsPathError {
    // TODO: normal path errors
    PermissionDenied,
    NoSuchFileOrDirectory,
    NotADirectory,
    ParseError(PathParseError),
}

impl FsPathError {
    pub fn into_errno(self) -> Errno {
        match self {
            FsPathError::NoSuchFileOrDirectory => ENOENT,
            FsPathError::NotADirectory => ENOTDIR,
            FsPathError::PermissionDenied => EACCES,
            FsPathError::ParseError(err) => err.as_errno(),
        }
    }
}

#[derive(Debug)]
pub enum FsReadError {}

#[derive(Debug)]
pub enum FsWriteError {}

#[derive(Debug)]
pub enum FsOpenError {
    BadPath(FsPathError),
}

#[derive(Debug)]
pub enum FsCloseError {}

#[derive(Debug)]
pub enum FsStatError {
    BadPath(FsPathError),
}

#[derive(Debug)]
pub enum FsIoctlError {}

#[derive(Debug)]
pub enum FsSeekError {}

#[derive(Debug)]
pub enum FsInitError {
    InvalidSkeleton,
    InvalidMagic,
    InvalidSuperBlock,
}

#[derive(Debug)]
pub enum FsMountError {
    BadPath(FsPathError),
    PathAlreadyInUse,
    FileSystemInitFailed(FsInitError),
}
