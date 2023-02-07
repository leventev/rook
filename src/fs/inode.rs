use alloc::fmt;

#[derive(PartialEq, Clone, Copy)]
pub struct FSInode(pub u64);

impl FSInode {
    pub const fn new(val: u64) -> FSInode {
        FSInode(val)
    }
}

impl fmt::Display for FSInode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
