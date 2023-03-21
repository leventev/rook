pub mod errno;

pub const AT_FCWD: isize = -100;

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
