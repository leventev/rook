pub mod kalloc;
pub mod phys;
pub mod virt;

use core::{fmt, ops};

use alloc::slice;

use crate::mm::virt::PAGE_ENTRIES;

use self::{
    phys::FRAME_SIZE,
    virt::{HHDM_START, PAGE_SIZE_4KIB},
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtAddr(u64);

impl VirtAddr {
    pub const fn pml4_index(&self) -> u64 {
        (self.0 >> 39) & 0o777
    }

    pub const fn pml3_index(&self) -> u64 {
        (self.0 >> 30) & 0o777
    }

    pub const fn pml2_index(&self) -> u64 {
        (self.0 >> 21) & 0o777
    }

    pub const fn pml1_index(&self) -> u64 {
        (self.0 >> 12) & 0o777
    }

    pub const fn get(&self) -> u64 {
        self.0
    }

    pub const fn page_offset(&self) -> u64 {
        self.0 % PAGE_SIZE_4KIB
    }

    pub const fn new(val: u64) -> VirtAddr {
        VirtAddr(val)
    }

    pub const fn zero() -> VirtAddr {
        VirtAddr(0)
    }
}

impl ops::Add<VirtAddr> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: VirtAddr) -> Self::Output {
        VirtAddr::new(self.get().checked_add(rhs.get()).unwrap())
    }
}

impl ops::Sub<VirtAddr> for VirtAddr {
    type Output = VirtAddr;

    fn sub(self, rhs: VirtAddr) -> Self::Output {
        VirtAddr::new(self.get().checked_sub(rhs.get()).unwrap())
    }
}

impl fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val, f)
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysAddr(u64);

impl<'a> PhysAddr {
    pub fn get(&self) -> u64 {
        self.0
    }

    pub const fn new(val: u64) -> PhysAddr {
        PhysAddr(val)
    }

    pub const fn zero() -> PhysAddr {
        PhysAddr(0)
    }

    pub fn is_aligned(&self) -> bool {
        self.0 as usize % FRAME_SIZE == 0
    }

    pub fn virt_addr(&self) -> VirtAddr {
        let hhdm_start = *HHDM_START.read();

        assert_ne!(hhdm_start, VirtAddr::zero());
        VirtAddr::new(hhdm_start.get() + self.0)
    }

    pub fn as_page_table(&self) -> &'a [u64] {
        unsafe { slice::from_raw_parts(self.virt_addr().get() as *const u64, PAGE_ENTRIES) }
    }

    pub fn as_mut_page_table(&self) -> &'a mut [u64] {
        unsafe { slice::from_raw_parts_mut(self.virt_addr().get() as *mut u64, PAGE_ENTRIES) }
    }
}

impl ops::Add<PhysAddr> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: PhysAddr) -> Self::Output {
        PhysAddr::new(self.get().checked_add(rhs.get()).unwrap())
    }
}

impl ops::Sub<PhysAddr> for PhysAddr {
    type Output = PhysAddr;

    fn sub(self, rhs: PhysAddr) -> Self::Output {
        PhysAddr::new(self.get().checked_sub(rhs.get()).unwrap())
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val, f)
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}
