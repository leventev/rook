use crate::arch::x86_64::get_cr3;
use crate::arch::x86_64::paging::PageFlags;
use crate::mm::{PhysAddr, VirtAddr};
use spin::Mutex;

use super::phys;

const PAGE_ENTRIES: u64 = 512;
const PAGE_KERNEL_PML4_INDEX: u64 = 511;
const VIRT_MANAGER_PML4_INDEX: u64 = 509;
const RECURSIVE_PML4_INDEX: u64 = 510;

// TODO: support other arches, and abstract all virtual memory operations
struct VirtualMemoryManager {
    pml4_phys: PhysAddr,
    initialized: bool,
}

impl VirtualMemoryManager {
    // Initializes the virtual memory manager
    pub fn init(&mut self, hhdm: VirtAddr) {
        self.pml4_phys = PhysAddr(unsafe { get_cr3() });
        let pml4_virt = hhdm + VirtAddr::new(self.pml4_phys.get());

        // cant use map_mpl4 here because the recursive mappings havent been
        // estabilished yet
        let pml4 = unsafe {
            core::slice::from_raw_parts_mut(pml4_virt.0 as *mut u64, PAGE_ENTRIES as usize)
        };
        pml4[RECURSIVE_PML4_INDEX as usize] =
            self.pml4_phys.get() | (PageFlags::READ_WRITE | PageFlags::PRESENT).bits();

        self.initialized = true;
    }

    /// Generates a memory address that can be used to access a page table
    fn get_recursive_addr(
        pml4_idx: u64,
        pml3_idx: u64,
        pml2_idx: u64,
        pml1_idx: u64,
        idx: u64,
    ) -> u64 {
        assert!(pml4_idx < PAGE_ENTRIES);
        assert!(pml3_idx < PAGE_ENTRIES);
        assert!(pml2_idx < PAGE_ENTRIES);
        assert!(pml1_idx < PAGE_ENTRIES);
        assert!(idx < 4096);

        (0xffff << 48)
            | (pml4_idx << 39)
            | (pml3_idx << 30)
            | (pml2_idx << 21)
            | (pml1_idx << 12)
            | (idx * 8)
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

            let ent = unsafe { *ptr };
            if ent == 0 {
                continue;
            }
        }
    }

    /// Returns the page table entry
    pub fn get_pml4(&self, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { *ptr }
    }

    /// Returns the page table entry
    pub fn get_pml3(&self, pml4_index: u64, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { *ptr }
    }

    /// Returns the page table entry
    pub fn get_pml2(&self, pml4_index: u64, pml3_index: u64, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { *ptr }
    }

    /// Returns the page table entry
    pub fn get_pml1(&self, pml4_index: u64, pml3_index: u64, pml2_index: u64, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            pml2_index,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { *ptr }
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    fn map_pml4(&self, index: u64, phys: PhysAddr, flags: PageFlags) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            index,
        );

        let ptr = addr as *mut u64;
        assert_eq!(unsafe { *ptr }, 0);

        let ent = phys.get() | flags.bits();
        unsafe { *ptr = ent };
        ent
    }

    /// If the entry at the specified index is empty, allocate a new phyiscal page
    /// then return the entry
    fn get_or_map_pml4(&self, index: u64) -> u64 {
        let mut ent = self.get_pml4(index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.map_pml4(index, phys, PageFlags::READ_WRITE | PageFlags::PRESENT);

            // zero out the page
            let start_addr = VirtualMemoryManager::get_recursive_addr(
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                index,
                0,
            );
            for i in 0..4096 / 8 {
                let ptr = (start_addr + i * 8) as *mut u64;
                unsafe {
                    *ptr = 0;
                }
            }

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml4 entry({}) was empty, allocated a new frame: {}",
                    index, phys
                );
            }
        }
        ent
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    fn map_pml3(&self, pml4_index: u64, index: u64, phys: PhysAddr, flags: PageFlags) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            index,
        );

        let ptr = addr as *mut u64;
        assert_eq!(unsafe { *ptr }, 0);

        let ent = phys.get() | flags.bits();
        unsafe { *ptr = ent };
        ent
    }

    /// If the entry at the specified index is empty, allocate a new phyiscal page
    /// then return the entry
    fn get_or_map_pml3(&self, pml4_index: u64, index: u64) -> u64 {
        let mut ent = self.get_pml3(pml4_index, index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.map_pml3(
                pml4_index,
                index,
                phys,
                PageFlags::READ_WRITE | PageFlags::PRESENT,
            );

            // zero out the page
            let start_addr = VirtualMemoryManager::get_recursive_addr(
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                pml4_index,
                index,
                0,
            );
            for i in 0..4096 / 8 {
                let ptr = (start_addr + i * 8) as *mut u64;
                unsafe {
                    *ptr = 0;
                }
            }

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml3 entry({}/{}) was empty, allocated a new frame: {}",
                    pml4_index, index, phys
                );
            }
        }
        ent
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    fn map_pml2(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        index: u64,
        phys: PhysAddr,
        flags: PageFlags,
    ) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            index,
        );

        let ptr = addr as *mut u64;
        assert_eq!(unsafe { *ptr }, 0);

        let ent = phys.get() | flags.bits();
        unsafe { *ptr = ent };
        ent
    }

    /// If the entry at the specified index is empty, allocate a new phyiscal page
    /// then return the entry
    fn get_or_map_pml2(&self, pml4_index: u64, pml3_index: u64, index: u64) -> u64 {
        let mut ent = self.get_pml2(pml4_index, pml3_index, index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.map_pml2(
                pml4_index,
                pml3_index,
                index,
                phys,
                PageFlags::READ_WRITE | PageFlags::PRESENT,
            );

            // zero out the page
            let start_addr = VirtualMemoryManager::get_recursive_addr(
                RECURSIVE_PML4_INDEX,
                pml4_index,
                pml3_index,
                index,
                0,
            );
            for i in 0..4 {
                let ptr = (start_addr + i * 8) as *mut u64;
                unsafe {
                    *ptr = 0;
                }
            }

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml2 entry({}/{}/{}) was empty, allocated a new frame: {}",
                    pml4_index, pml3_index, index, phys
                );
            }
        }
        ent
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    fn map_pml1(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        index: u64,
        phys: PhysAddr,
        flags: PageFlags,
    ) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            pml2_index,
            index,
        );

        let ptr = addr as *mut u64;
        assert_eq!(unsafe { *ptr }, 0);

        let ent = phys.get() | flags.bits();
        unsafe { *ptr = ent };
        ent
    }

    /// If the entry at the specified index is empty, allocates a new phyiscal page
    /// then returns the entry
    fn get_or_map_pml1(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        index: u64,
    ) -> u64 {
        let mut ent = self.get_pml1(pml4_index, pml3_index, pml2_index, index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.map_pml1(
                pml4_index,
                pml3_index,
                pml2_index,
                index,
                phys,
                PageFlags::READ_WRITE | PageFlags::PRESENT,
            );

            // zero out the page
            let start_addr = VirtualMemoryManager::get_recursive_addr(
                pml4_index, pml3_index, pml2_index, index, 0,
            );
            for i in 0..4096 / 8 {
                let ptr = (start_addr + i * 8) as *mut u64;
                unsafe {
                    *ptr = 0;
                }
            }

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml1 entry({}/{}/{}/{}) was empty, allocated a new frame: {}",
                    pml4_index, pml3_index, pml2_index, index, phys
                );
            }
        }
        ent
    }

    /// This function maps a page in virtual memory to physical memory
    /// Allocates the associated page tables if they are not present
    pub fn map(&self, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
        assert!(self.initialized);
        assert!(virt.get() % 4096 == 0);
        assert!(phys.get() % 4096 == 0);
        // TODO: check if address is valid

        let pml4_idx = virt.pml4_index();
        let pml3_idx = virt.pml3_index();
        let pml2_idx = virt.pml2_index();
        let pml1_idx = virt.pml1_index();

        // check if the page tables required are present, if not allocate them
        self.get_or_map_pml4(pml4_idx);
        self.get_or_map_pml3(pml4_idx, pml3_idx);
        self.get_or_map_pml2(pml4_idx, pml3_idx, pml2_idx);

        let page = self.get_pml1(pml4_idx, pml3_idx, pml2_idx, pml1_idx);
        assert!(page == 0, "Trying to map already mapped page!");

        self.map_pml1(
            pml4_idx,
            pml3_idx,
            pml2_idx,
            pml1_idx,
            phys,
            flags | PageFlags::PRESENT,
        );

        if cfg!(vmm_debug) {
            println!(
                "VMM: mapped Virt {} -> Phys {} with flags {:?}",
                virt, phys, flags
            );
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
            self.get_pml4(pml4_idx) != 0,
            "Trying to unmap unmapped page!"
        );
        assert!(
            self.get_pml3(pml4_idx, pml3_idx) != 0,
            "Trying to unmap unmapped page!"
        );
        assert!(
            self.get_pml2(pml4_idx, pml3_idx, pml2_idx) != 0,
            "Trying to unmap unmapped page!"
        );
        assert!(
            self.get_pml1(pml4_idx, pml3_idx, pml2_idx, pml1_idx) != 0,
            "Trying to unmap unmapped page!"
        );

        self.map_pml1(
            pml4_idx,
            pml3_idx,
            pml2_idx,
            pml1_idx,
            PhysAddr::zero(),
            PageFlags::NONE,
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

        let pml1 = self.get_pml1(pml4_idx, pml3_idx, pml2_idx, pml1_idx);

        PhysAddr::new(pml1 & 0xfffffffffffff000 + offset)
    }
}

static VIRTUAL_MEMORY_MANAGER: Mutex<VirtualMemoryManager> = Mutex::new(VirtualMemoryManager {
    pml4_phys: PhysAddr::zero(),
    initialized: false,
});

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

pub fn map(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map(virt, phys, flags);
}

pub fn unmap(virt: VirtAddr) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.unmap(virt);
}

pub fn get_phys_from_virt(virt: VirtAddr) -> PhysAddr {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.get_phys_from_virt(virt)
}
