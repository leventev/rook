use core::ops::Add;

use alloc::fmt;

pub struct Inode(pub u64);

impl Inode {
    pub const fn new(val: u64) -> Inode {
        Inode(val)
    }
}

impl fmt::Display for Inode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
