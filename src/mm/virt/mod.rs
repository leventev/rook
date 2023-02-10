use crate::arch::x86_64::get_cr3;
use crate::arch::x86_64::paging::{PML1Flags, PML2Flags, PML3Flags, PML4Flags, PageFlags};
use crate::mm::{PhysAddr, VirtAddr};
use spin::Mutex;

use super::phys;

mod recursive;

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

// TODO: support other arches, and abstract all virtual memory operations
struct VirtualMemoryManager {
    initialized: bool,
}

impl VirtualMemoryManager {
    // Initializes the virtual memory manager
    pub fn init(&mut self, hhdm: VirtAddr) {
        let pml4_phys = PhysAddr(unsafe { get_cr3() });
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

    /// Prints out the contents of the PML4
    pub fn dump_pml4(&self) {
        assert!(self.initialized);

        for i in 0..PAGE_ENTRIES {
            let addr = VirtualMemoryManager::get_recursive_addr(
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                i,
            );

            let ptr = addr as *mut u64;

            let ent = unsafe { ptr.read() };
            if ent == 0 {
                continue;
            }

            // TODO: pretty print
            println!("{}: {:#x}", i, ent);
        }
    }

    /// This function maps a 4KiB/4MiB page in virtual memory to physical memory
    /// Allocates the associated page tables if they are not present
    pub fn map(&self, virt: VirtAddr, phys: PhysAddr, flags: PageFlags, page_2mb: bool) {
        assert!(self.initialized);
        assert!(virt.get() % 4096 == 0);
        assert!(phys.get() % 4096 == 0);
        // TODO: check if address is valid

        let (pml4_flags, pml4_index) = (flags.to_plm4_flags(), virt.pml4_index());
        let (pml3_flags, pml3_index) = (flags.to_plm3_flags(), virt.pml3_index());

        // check if the page tables required are present, if not allocate them
        self.recursive_get_or_map_pml4(pml4_index, pml4_flags);
        self.recursive_get_or_map_pml3(pml4_index, pml3_index, pml3_flags);

        if page_2mb {
            let (pml2_flags, pml2_index) = (
                flags.to_plm2_flags() | PML2Flags::PAGE_SIZE,
                virt.pml2_index(),
            );

            let page = self.recursive_get_pml2(pml4_index, pml3_index, pml2_index);
            assert!(page == 0, "Trying to map already mapped page!");

            self.recursive_map_pml2(pml4_index, pml3_index, pml2_index, phys, pml2_flags);
        } else {
            let (pml2_flags, pml2_index) = (flags.to_plm2_flags(), virt.pml2_index());

            self.recursive_get_or_map_pml2(pml4_index, pml3_index, pml2_index, pml2_flags);

            let (pml1_flags, pml1_index) = (flags.to_plm1_flags(), virt.pml1_index());
            let page = self.recursive_get_pml1(pml4_index, pml3_index, pml2_index, pml1_index);
            assert!(page == 0, "Trying to map already mapped page!");

            self.recursive_map_pml1(
                pml4_index, pml3_index, pml2_index, pml1_index, phys, pml1_flags,
            );
        }

        if cfg!(vmm_debug) {
            println!("VMM: mapped Virt {} -> Phys {}", virt, phys);
        }
    }

    /// This function unmaps a page in virtual memory
    /// It does not deallocate the physical memory neither the page tables associated with it
    pub fn unmap(&self, virt: VirtAddr) {
        assert!(self.initialized);
        assert!(virt.get() % 4096 == 0);
        // TODO: check if address is valid

        let pml4_idx = virt.pml4_index();
        let pml3_idx = virt.pml3_index();
        let pml2_idx = virt.pml2_index();
        let pml1_idx = virt.pml1_index();

        // check if the page tables required are present, if not allocate them
        assert!(
            self.recursive_get_pml4(pml4_idx) != 0,
            "Trying to unmap unmapped page!"
        );
        assert!(
            self.recursive_get_pml3(pml4_idx, pml3_idx) != 0,
            "Trying to unmap unmapped page!"
        );
        assert!(
            self.recursive_get_pml2(pml4_idx, pml3_idx, pml2_idx) != 0,
            "Trying to unmap unmapped page!"
        );
        assert!(
            self.recursive_get_pml1(pml4_idx, pml3_idx, pml2_idx, pml1_idx) != 0,
            "Trying to unmap unmapped page!"
        );

        self.recursive_map_pml1(
            pml4_idx,
            pml3_idx,
            pml2_idx,
            pml1_idx,
            PhysAddr::zero(),
            PML1Flags::NONE,
        );

        if cfg!(vmm_debug) {
            println!("VMM: unmapped Virt {}", virt);
        }
    }

    fn get_phys_from_virt(&self, virt: VirtAddr) -> PhysAddr {
        let pml4_idx = virt.pml4_index();
        let pml3_idx = virt.pml3_index();
        let pml2_idx = virt.pml2_index();
        let pml1_idx = virt.pml1_index();

        let offset = virt.get() % 4096;

        let pml1 = self.recursive_get_pml1(pml4_idx, pml3_idx, pml2_idx, pml1_idx);

        PhysAddr::new(pml1 & 0xfffffffffffff000 + offset)
    }

    fn map_physical_address_space(&self) {
        const PAGES_TO_MAP: u64 = PAGE_ENTRIES * PAGE_ENTRIES;

        let pml4_index = PHYSICAL_ADDRESS_SPACE_VIRT_ADDR.pml4_index();
        assert_eq!(self.recursive_get_pml4(pml4_index), 0);

        // map here manually because the map_pmlX functions try to use the physical
        // address space mappings

        self.recursive_map_pml4(
            pml4_index,
            phys::alloc(),
            PML4Flags::READ_WRITE | PML4Flags::PRESENT,
        );

        for pml3_index in 0..PAGE_ENTRIES {
            self.recursive_map_pml3(
                pml4_index,
                pml3_index,
                phys::alloc(),
                PML3Flags::READ_WRITE | PML3Flags::PRESENT,
            );

            for pml2_index in 0..PAGE_ENTRIES {
                let phys_addr =
                    PhysAddr::new((pml3_index * PAGE_ENTRIES + pml2_index) * PAGE_SIZE_2MIB);

                self.recursive_map_pml2(
                    pml4_index,
                    pml3_index,
                    pml2_index,
                    phys_addr,
                    PML2Flags::READ_WRITE | PML2Flags::PRESENT,
                );
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

pub fn dump_pml4() {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.dump_pml4();
}

/// Maps a 4KiB page in memory
pub fn map_4kib(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(virt, phys, flags | PageFlags::PRESENT, false);
}

/// Maps a 2MiB page in memory
pub fn map_2mib(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(virt, phys, flags, true);
}

pub fn unmap(virt: VirtAddr) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.unmap(virt);
}

pub fn get_phys_from_virt(virt: VirtAddr) -> PhysAddr {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.get_phys_from_virt(virt)
}

// pml[507]
const PHYSICAL_ADDRESS_SPACE_VIRT_ADDR: VirtAddr = VirtAddr::new(0xfffffd8000000000);

/// This function maps 512GiB into memory
pub fn map_physical_address_space() {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map_physical_address_space();
}
