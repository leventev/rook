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

pub const F_DUPFD: usize = 0;
pub const F_GETFD: usize = 1;
pub const F_SETFD: usize = 2;
pub const F_GETFL: usize = 3;
pub const F_SETFL: usize = 4;

pub const TCGETS: usize = 0x5401;
pub const TCSETS: usize = 0x5402;
pub const TIOCGPGRP: usize = 0x540F;
pub const TIOCSPGRP: usize = 0x5410;

pub const VINTR: usize = 0;
pub const VQUIT: usize = 1;
pub const VERASE: usize = 2;
pub const VKILL: usize = 3;
pub const VEOF: usize = 4;
pub const VTIME: usize = 5;
pub const VMIN: usize = 6;
pub const VSWTC: usize = 7;
pub const VSTART: usize = 8;
pub const VSTOP: usize = 9;
pub const VSUSP: usize = 10;
pub const VEOL: usize = 11;
pub const VREPRINT: usize = 12;
pub const VDISCARD: usize = 13;
pub const VWERASE: usize = 14;
pub const VLNEXT: usize = 15;
pub const VEOL2: usize = 16;

pub const IGNBRK: usize = 0o000001;
pub const BRKINT: usize = 0o000002;
pub const IGNPAR: usize = 0o000004;
pub const PARMRK: usize = 0o000010;
pub const INPCK: usize = 0o000020;
pub const ISTRIP: usize = 0o000040;
pub const INLCR: usize = 0o000100;
pub const IGNCR: usize = 0o000200;
pub const ICRNL: usize = 0o000400;
pub const IUCLC: usize = 0o001000;
pub const IXON: usize = 0o002000;
pub const IXANY: usize = 0o004000;
pub const IXOFF: usize = 0o010000;
pub const IMAXBEL: usize = 0o020000;
pub const IUTF8: usize = 0o040000;

pub const VTDLY: usize = 0o040000;
pub const VT0: usize = 0o000000;
pub const VT1: usize = 0o040000;

pub const B0: usize = 0o000000;
pub const B50: usize = 0o000001;
pub const B75: usize = 0o000002;
pub const B110: usize = 0o000003;
pub const B134: usize = 0o000004;
pub const B150: usize = 0o000005;
pub const B200: usize = 0o000006;
pub const B300: usize = 0o000007;
pub const B600: usize = 0o000010;
pub const B1200: usize = 0o000011;
pub const B1800: usize = 0o000012;
pub const B2400: usize = 0o000013;
pub const B4800: usize = 0o000014;
pub const B9600: usize = 0o000015;
pub const B19200: usize = 0o000016;
pub const B38400: usize = 0o000017;

pub const B57600: usize = 0o010001;
pub const B115200: usize = 0o010002;
pub const B230400: usize = 0o010003;
pub const B460800: usize = 0o010004;
pub const B500000: usize = 0o010005;
pub const B576000: usize = 0o010006;
pub const B921600: usize = 0o010007;
pub const B1000000: usize = 0o010010;
pub const B1152000: usize = 0o010011;
pub const B1500000: usize = 0o010012;
pub const B2000000: usize = 0o010013;
pub const B2500000: usize = 0o010014;
pub const B3000000: usize = 0o010015;
pub const B3500000: usize = 0o010016;
pub const B4000000: usize = 0o010017;

pub const CSIZE: usize = 0o000060;
pub const CS5: usize = 0o000000;
pub const CS6: usize = 0o000020;
pub const CS7: usize = 0o000040;
pub const CS8: usize = 0o000060;
pub const CSTOPB: usize = 0o000100;
pub const CREAD: usize = 0o000200;
pub const PARENB: usize = 0o000400;
pub const PARODD: usize = 0o001000;
pub const HUPCL: usize = 0o002000;
pub const CLOCAL: usize = 0o004000;

pub const ECHO: usize = 0x1;
pub const ICANON: usize = 0x10;
pub const ISIG: usize = 0x40;

pub const ECHOE: usize = 0o000020;
pub const ECHOK: usize = 0o000040;
pub const ECHONL: usize = 0o000100;
pub const NOFLSH: usize = 0o000200;
pub const TOSTOP: usize = 0o000400;
pub const IEXTEN: usize = 0o100000;

pub const TCOOFF: usize = 0;
pub const TCOON: usize = 1;
pub const TCIOFF: usize = 2;
pub const TCION: usize = 3;

pub const TCIFLUSH: usize = 0;
pub const TCOFLUSH: usize = 1;
pub const TCIOFLUSH: usize = 2;

pub const TCSANOW: usize = 0;
pub const TCSADRAIN: usize = 1;
pub const TCSAFLUSH: usize = 2;

pub const NCCS: usize = 32;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_cc: [u8; NCCS],
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Timespec {
    pub tv_sec: u64,
    pub tv_nsec: u64,
}

#[repr(C, packed)]
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
