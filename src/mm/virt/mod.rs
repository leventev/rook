use crate::arch::x86_64::paging::{PML1Flags, PML2Flags, PML3Flags, PML4Flags, PageFlags};
use crate::arch::x86_64::get_current_pml4;
use crate::mm::{PhysAddr, VirtAddr};
use alloc::slice;
use spin::Mutex;

use super::phys;

mod utils;

/// pml4[507] - physical memory(512GiB)
/// pml4[508] - kernel thread stacks
/// pml4[509] - kernel heap
/// pml4[510] - recrusive
/// pml4[511] - kernel
const VIRT_MANAGER_PML4_INDEX: u64 = 509;
const RECURSIVE_PML4_INDEX: u64 = 510;
const PAGE_KERNEL_PML4_INDEX: u64 = 511;

pub const PAGE_ENTRIES: u64 = 512;

pub const PAGE_SIZE_4KIB: u64 = 4096;
pub const PAGE_SIZE_2MIB: u64 = PAGE_SIZE_4KIB * 512;

// pml[507]
pub const PHYSICAL_ADDRESS_SPACE_VIRT_ADDR: VirtAddr = VirtAddr::new(0xfffffd8000000000);

// TODO: support other arches, and abstract all virtual memory operations
struct VirtualMemoryManager {
    initialized: bool,
}

impl VirtualMemoryManager {
    // Initializes the virtual memory manager
    pub fn init(&mut self, hhdm: VirtAddr) {
        let pml4_phys = get_current_pml4();
        let pml4_virt = hhdm + VirtAddr::new(pml4_phys.get());

        // cant use map_mpl4 here because the recursive mappings havent been
        // estabilished yet
        let pml4 = unsafe {
            core::slice::from_raw_parts_mut(pml4_virt.0 as *mut u64, PAGE_ENTRIES as usize)
        };
        pml4[RECURSIVE_PML4_INDEX as usize] =
            pml4_phys.get() | (PML1Flags::READ_WRITE | PML1Flags::PRESENT).bits();

        self.initialized = true;
    }

    /// This function maps a 4KiB/4MiB page in virtual memory to physical memory
    /// Allocates the associated page tables if they are not present
    fn map(
        &self,
        pml4: PhysAddr,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: PageFlags,
        page_2mb: bool,
    ) {
        assert!(self.initialized);
        assert!(virt.get() % 4096 == 0);
        assert!(phys.get() % 4096 == 0);
        // TODO: check if address is valid

        let (pml4_flags, pml4_index) = (flags.to_plm4_flags(), virt.pml4_index());
        let (pml3_flags, pml3_index) = (flags.to_plm3_flags(), virt.pml3_index());

        // check if the page tables required are present, if not allocate them
        let pml3_table_phys = self.get_or_map_pml4(pml4, pml4_index, pml4_flags);
        let pml2_table_phys = self.get_or_map_pml3(pml3_table_phys, pml3_index, pml3_flags);

        if page_2mb {
            let (pml2_flags, pml2_index) = (
                flags.to_plm2_flags() | PML2Flags::PAGE_SIZE,
                virt.pml2_index(),
            );

            let page = self.get_pml2(pml2_table_phys, pml2_index);
            assert!(page.is_some(), "Trying to map already mapped page!");

            self.map_pml2(pml2_table_phys, pml2_index, phys, pml2_flags);
        } else {
            let (pml2_flags, pml2_index) = (flags.to_plm2_flags(), virt.pml2_index());
            let pml1_table_phys = self.get_or_map_pml2(pml2_table_phys, pml2_index, pml2_flags);

            let (pml1_flags, pml1_index) = (flags.to_plm1_flags(), virt.pml1_index());
            let page = self.get_pml1(pml1_table_phys, pml1_index);
            assert!(page.is_none(), "Trying to map already mapped page!");

            self.map_pml1(pml1_table_phys, pml1_index, phys, pml1_flags);
        }

        if cfg!(vmm_debug) {
            println!("VMM: mapped Virt {} -> Phys {}", virt, phys);
        }
    }

    /// This function unmaps a page in virtual memory
    /// It does not deallocate the physical memory neither the page tables associated with it
    fn unmap(&self, pml4_phys: PhysAddr, virt: VirtAddr) {
        assert!(self.initialized);
        assert!(virt.get() % 4096 == 0);
        // TODO: check if address is valid

        let pml4_idx = virt.pml4_index();
        let pml3_idx = virt.pml3_index();
        let pml2_idx = virt.pml2_index();
        let pml1_idx = virt.pml1_index();

        // check if the page tables required are present, if not allocate them
        let pml4 = self
            .get_pml4(pml4_phys, pml4_idx)
            .expect("Trying to unmap a not mapped page!");
        let pml3 = self
            .get_pml3(pml4.0, pml3_idx)
            .expect("Trying to unmap a not mapped page!");
        let pml2 = self
            .get_pml2(pml3.0, pml2_idx)
            .expect("Trying to unmap a not mapped page!");
        let pml1 = self
            .get_pml1(pml2.0, pml1_idx)
            .expect("Trying to unmap a not mapped page!");

        self.map_pml1(pml1.0, pml1_idx, PhysAddr::zero(), PML1Flags::NONE);

        if cfg!(vmm_debug) {
            println!("VMM: unmapped Virt {}", virt);
        }
    }

    fn get_recursive_addr(
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        pml1_index: u64,
        index: u64,
    ) -> u64 {
        assert!(pml4_index < PAGE_ENTRIES);
        assert!(pml3_index < PAGE_ENTRIES);
        assert!(pml2_index < PAGE_ENTRIES);
        assert!(pml1_index < PAGE_ENTRIES);
        assert!(index < 4096);

        (0xffff << 48)
            | (pml4_index << 39)
            | (pml3_index << 30)
            | (pml2_index << 21)
            | (pml1_index << 12)
            | (index * 8)
    }

    fn get_phys_from_virt(&self, pml4_phys: PhysAddr, virt: VirtAddr) -> PhysAddr {
        let pml4_idx = virt.pml4_index();
        let pml3_idx = virt.pml3_index();
        let pml2_idx = virt.pml2_index();
        let pml1_idx = virt.pml1_index();

        let offset = virt.get() % 4096;

        let pml4 = self.get_pml4(pml4_phys, pml4_idx).unwrap();
        let pml3 = self.get_pml4(pml4.0, pml3_idx).unwrap();
        let pml2 = self.get_pml4(pml3.0, pml2_idx).unwrap();
        let pml1 = self.get_pml1(pml2.0, pml1_idx).unwrap();

        PhysAddr::new(pml1.0.get() & 0xfffffffffffff000 + offset)
    }

    fn map_physical_address_space(&self) {
        const PAGES_TO_MAP: u64 = PAGE_ENTRIES * PAGE_ENTRIES;

        let pml4_index = PHYSICAL_ADDRESS_SPACE_VIRT_ADDR.pml4_index();

        let pml4_addr = Self::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
        ) as *mut u64;

        unsafe {
            pml4_addr
                .write(phys::alloc().get() | (PML4Flags::READ_WRITE | PML4Flags::PRESENT).bits())
        };

        for pml3_index in 0..PAGE_ENTRIES {
            let pml3_addr = Self::get_recursive_addr(
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                pml4_index,
                pml3_index,
            ) as *mut u64;

            unsafe {
                pml3_addr.write(
                    phys::alloc().get() | (PML3Flags::READ_WRITE | PML3Flags::PRESENT).bits(),
                )
            };

            for pml2_index in 0..PAGE_ENTRIES {
                let phys_addr =
                    PhysAddr::new((pml3_index * PAGE_ENTRIES + pml2_index) * PAGE_SIZE_2MIB);

                let pml3_addr = Self::get_recursive_addr(
                    RECURSIVE_PML4_INDEX,
                    RECURSIVE_PML4_INDEX,
                    pml4_index,
                    pml3_index,
                    pml2_index,
                ) as *mut u64;

                unsafe {
                    pml3_addr.write(
                        phys_addr.get()
                            | (PML2Flags::READ_WRITE | PML2Flags::PRESENT | PML2Flags::PAGE_SIZE)
                                .bits(),
                    )
                };
            }
        }
    }
}

static VIRTUAL_MEMORY_MANAGER: Mutex<VirtualMemoryManager> =
    Mutex::new(VirtualMemoryManager { initialized: false });

extern "C" {
    static __kernel_start: u64;
    static __kernel_end: u64;
}

pub fn init(hhdm: u64) {
    let mut vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.init(VirtAddr(hhdm));
}

/// Maps a 4KiB page in memory
pub fn map_4kib(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(
        get_current_pml4(),
        virt,
        phys,
        flags | PageFlags::PRESENT,
        false,
    );
}

/// Maps a 2MiB page in memory
pub fn map_2mib(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(get_current_pml4(), virt, phys, flags, true);
}

pub fn map_2mib_other(pml4: PhysAddr, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(pml4, virt, phys, flags, true);
}

pub fn map_4kib_other(pml4: PhysAddr, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(pml4, virt, phys, flags | PageFlags::PRESENT, false);
}

pub fn unmap(virt: VirtAddr) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.unmap(get_current_pml4(), virt);
}

pub fn unmap_other(pml4: PhysAddr, virt: VirtAddr) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.unmap(pml4, virt);
}

pub fn get_phys_from_virt(virt: VirtAddr) -> PhysAddr {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.get_phys_from_virt(get_current_pml4(), virt)
}

pub fn get_phys_from_virt_other(pml4: PhysAddr, virt: VirtAddr) -> PhysAddr {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.get_phys_from_virt(pml4, virt)
}

/// This function maps 512GiB into memory
pub fn map_physical_address_space() {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map_physical_address_space();
}

pub fn copy_pml4_higher_half_entries(to: PhysAddr, from: PhysAddr) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();

    let pml4 = unsafe { slice::from_raw_parts_mut(to.get() as *mut u64, PAGE_ENTRIES as usize) };

    // indexes explained above
    // TODO: use constants
    pml4[507] = {
        let ent = vmm.get_pml4(from, 507).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[508] = {
        let ent = vmm.get_pml4(from, 508).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[509] = {
        let ent = vmm.get_pml4(from, 509).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[510] = {
        let ent = vmm.get_pml4(from, 510).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[511] = {
        let ent = vmm.get_pml4(from, 511).unwrap();
        ent.0.get() | ent.1.bits()
    };
}
