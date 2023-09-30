// sadly we can't make make this an enum because EWOULDBLOCK is defined as EAGAIN
// and i don't want to create my own list

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Errno(usize);

impl Errno {
    pub fn into_inner(self) -> usize {
        self.0
    }

    pub fn into_inner_result(self) -> isize {
        -(self.0 as isize)
    }
}

pub const ETOOBIG: Errno = Errno(1);
pub const EACCES: Errno = Errno(2);
pub const EADDRINUSE: Errno = Errno(3);
pub const EADDRNOTAVAIL: Errno = Errno(4);
pub const EAFNOSUPPORT: Errno = Errno(5);
pub const EAGAIN: Errno = Errno(6);
pub const EALREADY: Errno = Errno(7);
pub const EBADF: Errno = Errno(8);
pub const EBADMSG: Errno = Errno(9);
pub const EBUSY: Errno = Errno(10);
pub const ECANCELED: Errno = Errno(11);
pub const ECHILD: Errno = Errno(12);
pub const ECONNABORTED: Errno = Errno(13);
pub const ECONNREFUSED: Errno = Errno(14);
pub const ECONNRESET: Errno = Errno(15);
pub const EDEADLK: Errno = Errno(16);
pub const EDESTADDRREQ: Errno = Errno(17);
pub const EDOM: Errno = Errno(18);
pub const EDQUOT: Errno = Errno(19);
pub const EEXIST: Errno = Errno(20);
pub const EFAULT: Errno = Errno(21);
pub const EFBIG: Errno = Errno(22);
pub const EHOSTUNREACH: Errno = Errno(23);
pub const EIDRM: Errno = Errno(24);
pub const EILSEQ: Errno = Errno(25);
pub const EINPROGRESS: Errno = Errno(26);
pub const EINTR: Errno = Errno(27);
pub const EINVAL: Errno = Errno(28);
pub const EIO: Errno = Errno(29);
pub const EISCONN: Errno = Errno(30);
pub const EISDIR: Errno = Errno(31);
pub const ELOOP: Errno = Errno(32);
pub const EMFILE: Errno = Errno(33);
pub const EMLINK: Errno = Errno(34);
pub const EMSGSIZE: Errno = Errno(35);
pub const EMULTIHOP: Errno = Errno(36);
pub const ENAMETOOLONG: Errno = Errno(37);
pub const ENETDOWN: Errno = Errno(38);
pub const ENETRESET: Errno = Errno(39);
pub const ENETUNREACH: Errno = Errno(40);
pub const ENFILE: Errno = Errno(41);
pub const ENOBUFS: Errno = Errno(42);
pub const ENODATA: Errno = Errno(43);
pub const ENODEV: Errno = Errno(44);
pub const ENOENT: Errno = Errno(45);
pub const ENOEXEC: Errno = Errno(46);
pub const ENOLCK: Errno = Errno(47);
pub const ENOLINK: Errno = Errno(48);
pub const ENOMEM: Errno = Errno(49);
pub const ENOMSG: Errno = Errno(50);
pub const ENOPROTOOPT: Errno = Errno(51);
pub const ENOSPC: Errno = Errno(52);
pub const ENOSR: Errno = Errno(53);
pub const ENOSTR: Errno = Errno(54);
pub const ENOSYS: Errno = Errno(55);
pub const ENOTCONN: Errno = Errno(56);
pub const ENOTDIR: Errno = Errno(57);
pub const ENOTEMPTY: Errno = Errno(58);
pub const ENOTRECOVERABLE: Errno = Errno(59);
pub const ENOTSOCK: Errno = Errno(60);
pub const ENOTSUP: Errno = Errno(61);
pub const ENOTTY: Errno = Errno(62);
pub const ENXIO: Errno = Errno(63);
pub const EOPNOTSUPP: Errno = Errno(64);
pub const EOVERFLOW: Errno = Errno(65);
pub const EOWNERDEAD: Errno = Errno(66);
pub const EPERM: Errno = Errno(67);
pub const EPIPE: Errno = Errno(68);
pub const EPROTO: Errno = Errno(69);
pub const EPROTONOSUPPORT: Errno = Errno(70);
pub const EPROTOTYPE: Errno = Errno(71);
pub const ERANGE: Errno = Errno(72);
pub const EROFS: Errno = Errno(73);
pub const ESPIPE: Errno = Errno(74);
pub const ESRCH: Errno = Errno(75);
pub const ESTALE: Errno = Errno(76);
pub const ETIME: Errno = Errno(77);
pub const ETIMEDOUT: Errno = Errno(78);
pub const ETXTBSY: Errno = Errno(79);
pub const EWOULDBLOCK: Errno = Errno(80);
pub const EXDEV: Errno = Errno(81);
