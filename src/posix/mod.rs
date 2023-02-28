pub mod errno;

pub const AT_FCWD: isize = -100;

bitflags::bitflags! {
    // TODO
    pub struct FileOpenMode: u32 {
        const NONE = 0;
    }

    pub struct FileOpenFlags: u32 {
        const NONE = 0;
    }
}
