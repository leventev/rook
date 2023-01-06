bitflags::bitflags! {
    pub struct PageFlags: u64 {
        const NONE = 0;
        const PRESENT = 1 << 0;
        const READ_WRITE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const PAGE_ATTRIBUTE_TABLE = 1 << 7;
        const GLOBAL = 1 << 8;
    }
}
