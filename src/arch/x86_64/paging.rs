bitflags::bitflags! {
    /// Common flags
    pub struct PageFlags: u64 {
        const NONE = 0;
        const PRESENT = 1 << 0;
        const READ_WRITE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const ALLOC_ON_ACCESS = 1 << 9;
    }

    pub struct PML1Flags: u64 {
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
        const ALLOC_ON_ACCESS = 1 << 9;
    }

    pub struct PML2Flags: u64 {
        const NONE = 0;
        const PRESENT = 1 << 0;
        const READ_WRITE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const PAGE_SIZE = 1 << 7;
        const ALLOC_ON_ACCESS = 1 << 9;
    }

    pub struct PML3Flags: u64 {
        const NONE = 0;
        const PRESENT = 1 << 0;
        const READ_WRITE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const PAGE_SIZE = 1 << 7;
        const ALLOC_ON_ACCESS = 1 << 9;
    }

    pub struct PML4Flags: u64 {
        const NONE = 0;
        const PRESENT = 1 << 0;
        const READ_WRITE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const ALLOC_ON_ACCESS = 1 << 9;
    }
}

impl PageFlags {
    pub fn to_plm1_flags(&self) -> PML1Flags {
        PML1Flags::from_bits(self.bits).unwrap()
    }

    pub fn to_plm2_flags(&self) -> PML2Flags {
        PML2Flags::from_bits(self.bits).unwrap()
    }

    pub fn to_plm3_flags(&self) -> PML3Flags {
        PML3Flags::from_bits(self.bits).unwrap()
    }

    pub fn to_plm4_flags(&self) -> PML4Flags {
        PML4Flags::from_bits(self.bits).unwrap()
    }
}
