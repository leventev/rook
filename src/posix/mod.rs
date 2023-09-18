use crate::fs::FileType;

pub mod errno;
pub mod termios;

bitflags::bitflags! {
    // TODO
    pub struct FileOpenMode: u32 {
        const NONE = 0;
    }

    pub struct FileOpenFlags: u32 {
        const O_RDONLY = 0;
        const O_WRONLY = 1;
        const O_RDWR = 2;
        const O_CREAT = 1 << 6;
        const O_EXCL = 1 << 7;
        const O_NOCTTY = 1 << 8;
        const O_TRUNC = 1 << 9;
        const O_APPEND = 1 << 10;
        const O_NONBLOCK = 1 << 11;
        const O_DSYNC = 1 << 12;
        const O_SYNC = 1 << 13;
        const O_RSYNC = 1 << 14;
        const O_DIRECTORY = 1 << 15;
        const O_NOFOLLOW = 1 << 16;
        const O_CLOEXEC = 1 << 17;
    }
}

pub const F_DUPFD: usize = 1;
pub const F_DUPFD_CLOEXEC: usize = 2;
pub const F_GETFD: usize = 3;
pub const F_SETFD: usize = 4;
pub const F_GETFL: usize = 5;
pub const F_SETFL: usize = 6;
pub const F_GETLK: usize = 7;
pub const F_SETLK: usize = 8;
pub const F_SETLKW: usize = 9;
pub const F_GETOWN: usize = 10;
pub const F_SETOWN: usize = 11;

pub const S_IFMT: u32 = 0o170000;

pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFIFO: u32 = 0o010000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFSOCK: u32 = 0o140000;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Timespec {
    pub tv_sec: u64,
    pub tv_nsec: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Timeval {
    pub tv_sec: u64,
    pub tv_usec: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: u64,
    pub st_atim: Timespec,
    pub st_mtim: Timespec,
    pub st_ctim: Timespec,
    pub st_blksize: u64,
    pub st_blocks: u64,
}

impl Stat {
    pub const fn zero() -> Stat {
        Self {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            st_size: 0,
            st_atim: Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_mtim: Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_ctim: Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_blksize: 0,
            st_blocks: 0,
        }
    }

    pub const fn file_type(&self) -> FileType {
        if self.st_mode & S_IFDIR > 0 {
            FileType::Directory
        } else if self.st_mode & S_IFREG > 0 {
            FileType::RegularFile
        } else if self.st_mode & S_IFCHR > 0 {
            FileType::CharacterDevice
        } else if self.st_mode & S_IFBLK > 0 {
            FileType::BlockDevice
        } else if self.st_mode & S_IFIFO > 0 {
            FileType::FIFO
        } else if self.st_mode & S_IFLNK > 0 {
            FileType::Link
        } else if self.st_mode & S_IFSOCK > 0 {
            FileType::Socket
        } else {
            todo!()
        }
    }
}
