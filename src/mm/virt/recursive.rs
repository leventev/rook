use crate::{
    arch::x86_64::paging::{PML1Flags, PML2Flags, PML3Flags, PML4Flags},
    mm::{
        phys,
        virt::{PAGE_ENTRIES, RECURSIVE_PML4_INDEX},
        PhysAddr,
    },
};

use super::VirtualMemoryManager;

fn zero_page(table: *mut u64) {
    for i in 0..4096 / 8 {
        unsafe {
            table.offset(i).write(0);
        }
    }
}

impl VirtualMemoryManager {
    /// Generates a memory address that can be used to access a page table
    pub fn get_recursive_addr(
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

    fn zero_page_table(
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        pml1_index: u64,
        index: u64,
    ) {
        let addr = Self::get_recursive_addr(pml4_index, pml3_index, pml2_index, pml1_index, index);
        zero_page(addr as *mut u64);
    }

    /// Returns the page table entry
    pub fn recursive_get_pml4(&self, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { ptr.read() }
    }

    /// Returns the page table entry
    pub fn recursive_get_pml3(&self, pml4_index: u64, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { ptr.read() }
    }

    /// Returns the page table entry
    pub fn recursive_get_pml2(&self, pml4_index: u64, pml3_index: u64, index: u64) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            index,
        );

        let ptr = addr as *mut u64;
        unsafe { ptr.read() }
    }

    /// Returns the page table entry
    pub fn recursive_get_pml1(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        index: u64,
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
        unsafe { ptr.read() }
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    pub fn recursive_map_pml4(&self, index: u64, phys: PhysAddr, flags: PML4Flags) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            index,
        );

        let ent = phys.get() | flags.bits();
        unsafe {
            let ptr = addr as *mut u64;
            ptr.write(ent);
        }

        ent
    }

    /// If the entry at the specified index is empty, allocate a new phyiscal page
    /// then return the entry
    pub fn recursive_get_or_map_pml4(&self, index: u64, flags: PML4Flags) -> u64 {
        let mut ent = self.recursive_get_pml4(index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.recursive_map_pml4(index, phys, flags);

            Self::zero_page_table(
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                index,
                0,
            );

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml4 entry({}) was empty, allocated a new frame: {}",
                    index, phys
                );
            }
        } else {
            // check if the entry flags match
            let ent_flags = PML4Flags::from_bits(ent & 0xFFF).unwrap();
            assert!(ent_flags.difference(flags) != PML4Flags::DIRTY);
        }
        ent
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    pub fn recursive_map_pml3(
        &self,
        pml4_index: u64,
        index: u64,
        phys: PhysAddr,
        flags: PML3Flags,
    ) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            index,
        );

        let ent = phys.get() | flags.bits();
        unsafe {
            let ptr = addr as *mut u64;
            ptr.write(ent);
        }

        ent
    }

    /// If the entry at the specified index is empty, allocate a new phyiscal page
    /// then return the entry
    pub fn recursive_get_or_map_pml3(&self, pml4_index: u64, index: u64, flags: PML3Flags) -> u64 {
        let mut ent = self.recursive_get_pml3(pml4_index, index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.recursive_map_pml3(pml4_index, index, phys, flags);

            // zero out the page
            Self::zero_page_table(
                RECURSIVE_PML4_INDEX,
                RECURSIVE_PML4_INDEX,
                pml4_index,
                index,
                0,
            );

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml3 entry({}/{}) was empty, allocated a new frame: {}",
                    pml4_index, index, phys
                );
            }
        } else {
            // check if the entry flags match
            let ent_flags = PML3Flags::from_bits(ent & 0xFFF).unwrap();
            assert!(ent_flags.difference(flags) != PML3Flags::DIRTY);
        }
        ent
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    pub fn recursive_map_pml2(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        index: u64,
        phys: PhysAddr,
        flags: PML2Flags,
    ) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            index,
        );

        let ent = phys.get() | flags.bits();
        unsafe {
            let ptr = addr as *mut u64;
            ptr.write(ent);
        }

        ent
    }

    /// If the entry at the specified index is empty, allocate a new phyiscal page
    /// then return the entry
    pub fn recursive_get_or_map_pml2(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        index: u64,
        flags: PML2Flags,
    ) -> u64 {
        let mut ent = self.recursive_get_pml2(pml4_index, pml3_index, index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.recursive_map_pml2(pml4_index, pml3_index, index, phys, flags);

            // zero out the page
            Self::zero_page_table(RECURSIVE_PML4_INDEX, pml4_index, pml3_index, index, 0);

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml2 entry({}/{}/{}) was empty, allocated a new frame: {}",
                    pml4_index, pml3_index, index, phys
                );
            }
        } else {
            // check if the entry flags match
            let ent_flags = PML2Flags::from_bits(ent & 0xFFF).unwrap();
            assert!(ent_flags.difference(flags) != PML2Flags::DIRTY);
        }
        ent
    }

    /// Returns the written entry
    /// If phys 0 and flags 0 are passed the page is essentially unmapped
    pub fn recursive_map_pml1(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        index: u64,
        phys: PhysAddr,
        flags: PML1Flags,
    ) -> u64 {
        assert!(self.initialized);

        let addr = VirtualMemoryManager::get_recursive_addr(
            RECURSIVE_PML4_INDEX,
            pml4_index,
            pml3_index,
            pml2_index,
            index,
        );

        let ent = phys.get() | flags.bits();
        unsafe {
            let ptr = addr as *mut u64;
            ptr.write(ent);
        }

        ent
    }

    /// If the entry at the specified index is empty, allocates a new phyiscal page
    /// then returns the entry
    pub fn recursive_get_or_map_pml1(
        &self,
        pml4_index: u64,
        pml3_index: u64,
        pml2_index: u64,
        index: u64,
        flags: PML1Flags,
    ) -> u64 {
        let mut ent = self.recursive_get_pml1(pml4_index, pml3_index, pml2_index, index);
        if ent == 0 {
            let phys = phys::alloc();
            ent = self.recursive_map_pml1(pml4_index, pml3_index, pml2_index, index, phys, flags);

            Self::zero_page_table(pml4_index, pml3_index, pml2_index, index, 0);

            if cfg!(vmm_debug) {
                println!(
                    "VMM: pml1 entry({}/{}/{}/{}) was empty, allocated a new frame: {}",
                    pml4_index, pml3_index, pml2_index, index, phys
                );
            }
        } else {
            // check if the entry flags match
            let ent_flags = PML1Flags::from_bits(ent & 0xFFF).unwrap();
            assert!(ent_flags.difference(flags) != PML1Flags::DIRTY);
        }
        ent
    }
}
