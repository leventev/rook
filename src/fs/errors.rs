use crate::posix::errno::{Errno, EACCES, ENOENT, ENOTDIR};

use super::path::PathParseError;

#[derive(Debug)]
pub enum FsPathError {
    // TODO: normal path errors
    PermissionDenied,
    NoSuchFileOrDirectory,
    NotADirectory,
    ParseError(PathParseError),
}

impl Into<Errno> for FsPathError {
    fn into(self) -> Errno {
        match self {
            FsPathError::NoSuchFileOrDirectory => ENOENT,
            FsPathError::NotADirectory => ENOTDIR,
            FsPathError::PermissionDenied => EACCES,
            FsPathError::ParseError(err) => err.into(),
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
