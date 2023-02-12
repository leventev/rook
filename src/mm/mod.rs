pub mod kalloc;
pub mod phys;
pub mod virt;

use core::{fmt, ops};

use self::virt::HHDM_START;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtAddr(u64);

impl VirtAddr {
    pub fn pml4_index(&self) -> u64 {
        (self.0 >> 39) & 0o777
    }

    pub fn pml3_index(&self) -> u64 {
        (self.0 >> 30) & 0o777
    }

    pub fn pml2_index(&self) -> u64 {
        (self.0 >> 21) & 0o777
    }

    pub fn pml1_index(&self) -> u64 {
        (self.0 >> 12) & 0o777
    }

    pub fn get(&self) -> u64 {
        self.0
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
        return VirtAddr::new(self.get().checked_add(rhs.get()).unwrap());
    }
}

impl ops::Sub<VirtAddr> for VirtAddr {
    type Output = VirtAddr;

    fn sub(self, rhs: VirtAddr) -> Self::Output {
        return VirtAddr::new(self.get().checked_sub(rhs.get()).unwrap());
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

impl PhysAddr {
    pub fn get(&self) -> u64 {
        self.0
    }

    pub const fn new(val: u64) -> PhysAddr {
        PhysAddr(val)
    }

    pub const fn zero() -> PhysAddr {
        PhysAddr(0)
    }

    pub fn virt_addr(&self) -> VirtAddr {
        let hhdm_start = *HHDM_START.read();

        assert_ne!(hhdm_start, VirtAddr::zero());
        VirtAddr::new(hhdm_start.get() + self.0)
    }
}

impl ops::Add<PhysAddr> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: PhysAddr) -> Self::Output {
        return PhysAddr::new(self.get().checked_add(rhs.get()).unwrap());
    }
}

impl ops::Sub<PhysAddr> for PhysAddr {
    type Output = PhysAddr;

    fn sub(self, rhs: PhysAddr) -> Self::Output {
        return PhysAddr::new(self.get().checked_sub(rhs.get()).unwrap());
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
