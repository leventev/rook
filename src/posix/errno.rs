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

pub const EDOM: Errno = Errno(1);
pub const EILSEQ: Errno = Errno(2);
pub const ERANGE: Errno = Errno(3);
pub const E2BIG: Errno = Errno(1001);
pub const EACCES: Errno = Errno(1002);
pub const EADDRINUSE: Errno = Errno(1003);
pub const EADDRNOTAVAIL: Errno = Errno(1004);
pub const EAFNOSUPPORT: Errno = Errno(1005);
pub const EAGAIN: Errno = Errno(1006);
pub const EALREADY: Errno = Errno(1007);
pub const EBADF: Errno = Errno(1008);
pub const EBADMSG: Errno = Errno(1009);
pub const EBUSY: Errno = Errno(1010);
pub const ECANCELED: Errno = Errno(1011);
pub const ECHILD: Errno = Errno(1012);
pub const ECONNABORTED: Errno = Errno(1013);
pub const ECONNREFUSED: Errno = Errno(1014);
pub const ECONNRESET: Errno = Errno(1015);
pub const EDEADLK: Errno = Errno(1016);
pub const EDESTADDRREQ: Errno = Errno(1017);
pub const EDQUOT: Errno = Errno(1018);
pub const EEXIST: Errno = Errno(1019);
pub const EFAULT: Errno = Errno(1020);
pub const EFBIG: Errno = Errno(1021);
pub const EHOSTUNREACH: Errno = Errno(1022);
pub const EIDRM: Errno = Errno(1023);
pub const EINPROGRESS: Errno = Errno(1024);
pub const EINTR: Errno = Errno(1025);
pub const EINVAL: Errno = Errno(1026);
pub const EIO: Errno = Errno(1027);
pub const EISCONN: Errno = Errno(1028);
pub const EISDIR: Errno = Errno(1029);
pub const ELOOP: Errno = Errno(1030);
pub const EMFILE: Errno = Errno(1031);
pub const EMLINK: Errno = Errno(1032);
pub const EMSGSIZE: Errno = Errno(1034);
pub const EMULTIHOP: Errno = Errno(1035);
pub const ENAMETOOLONG: Errno = Errno(1036);
pub const ENETDOWN: Errno = Errno(1037);
pub const ENETRESET: Errno = Errno(1038);
pub const ENETUNREACH: Errno = Errno(1039);
pub const ENFILE: Errno = Errno(1040);
pub const ENOBUFS: Errno = Errno(1041);
pub const ENODEV: Errno = Errno(1042);
pub const ENOENT: Errno = Errno(1043);
pub const ENOEXEC: Errno = Errno(1044);
pub const ENOLCK: Errno = Errno(1045);
pub const ENOLINK: Errno = Errno(1046);
pub const ENOMEM: Errno = Errno(1047);
pub const ENOMSG: Errno = Errno(1048);
pub const ENOPROTOOPT: Errno = Errno(1049);
pub const ENOSPC: Errno = Errno(1050);
pub const ENOSYS: Errno = Errno(1051);
pub const ENOTCONN: Errno = Errno(1052);
pub const ENOTDIR: Errno = Errno(1053);
pub const ENOTEMPTY: Errno = Errno(1054);
pub const ENOTRECOVERABLE: Errno = Errno(1055);
pub const ENOTSOCK: Errno = Errno(1056);
pub const ENOTSUP: Errno = Errno(1057);
pub const ENOTTY: Errno = Errno(1058);
pub const ENXIO: Errno = Errno(1059);
pub const EOPNOTSUPP: Errno = Errno(1060);
pub const EOVERFLOW: Errno = Errno(1061);
pub const EOWNERDEAD: Errno = Errno(1062);
pub const EPERM: Errno = Errno(1063);
pub const EPIPE: Errno = Errno(1064);
pub const EPROTO: Errno = Errno(1065);
pub const EPROTONOSUPPORT: Errno = Errno(1066);
pub const EPROTOTYPE: Errno = Errno(1067);
pub const EROFS: Errno = Errno(1068);
pub const ESPIPE: Errno = Errno(1069);
pub const ESRCH: Errno = Errno(1070);
pub const ESTALE: Errno = Errno(1071);
pub const ETIMEDOUT: Errno = Errno(1072);
pub const ETXTBSY: Errno = Errno(1073);
pub const EWOULDBLOCK: Errno = EAGAIN;
pub const EXDEV: Errno = Errno(1075);
pub const ENODATA: Errno = Errno(1076);
pub const ETIME: Errno = Errno(1077);
pub const ENOKEY: Errno = Errno(1078);
pub const ESHUTDOWN: Errno = Errno(1079);
pub const EHOSTDOWN: Errno = Errno(1080);
pub const EBADFD: Errno = Errno(1081);
pub const ENOMEDIUM: Errno = Errno(1082);
pub const ENOTBLK: Errno = Errno(1083);
pub const ENONET: Errno = Errno(1084);
pub const EPFNOSUPPORT: Errno = Errno(1085);
pub const ESOCKTNOSUPPORT: Errno = Errno(1086);
pub const ESTRPIPE: Errno = Errno(1087);
pub const EREMOTEIO: Errno = Errno(1088);
pub const ERFKILL: Errno = Errno(1089);
pub const EBADR: Errno = Errno(1090);
pub const EUNATCH: Errno = Errno(1091);
pub const EMEDIUMTYPE: Errno = Errno(1092);
pub const EREMOTE: Errno = Errno(1093);
pub const EKEYREJECTED: Errno = Errno(1094);
pub const EUCLEAN: Errno = Errno(1095);
pub const EBADSLT: Errno = Errno(1096);
pub const ENOANO: Errno = Errno(1097);
pub const ENOCSI: Errno = Errno(1098);
pub const ENOSTR: Errno = Errno(1099);
pub const ETOOMANYREFS: Errno = Errno(1100);
pub const ENOPKG: Errno = Errno(1101);
pub const EKEYREVOKED: Errno = Errno(1102);
pub const EXFULL: Errno = Errno(1103);
pub const ELNRNG: Errno = Errno(1104);
pub const ENOTUNIQ: Errno = Errno(1105);
pub const ERESTART: Errno = Errno(1106);
pub const EUSERS: Errno = Errno(1107);
pub const ECHRNG: Errno = Errno(1108);
pub const ELIBBAD: Errno = Errno(1109);
pub const EL2HLT: Errno = Errno(1110);
pub const EL3HLT: Errno = Errno(1111);
pub const EKEYEXPIRED: Errno = Errno(1112);
pub const ECOMM: Errno = Errno(1113);
pub const EBADE: Errno = Errno(1114);
pub const EHWPOISON: Errno = Errno(1115);
pub const EBADRQC: Errno = Errno(1116);
