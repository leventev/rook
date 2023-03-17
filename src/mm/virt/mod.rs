use crate::arch::x86_64::paging::{PML1Flags, PML2Flags, PML3Flags, PML4Flags, PageFlags};
use crate::arch::x86_64::{flush_tlb_page, get_current_pml4, set_cr3};
use crate::mm::{PhysAddr, VirtAddr};
use alloc::slice;
use spin::{Mutex, RwLock};

mod utils;

/// pml4[508] - physical memory(512GiB)
/// pml4[509] - kernel thread stacks
/// pml4[510] - kernel heap
/// pml4[511] - kernel

// pml[508]
pub const HDDM_VIRT_START: VirtAddr = VirtAddr::new(0xfffffe0000000000);

// pml4[509]
pub const KERNEL_THREAD_STACKS_START: VirtAddr = VirtAddr::new(0xfffffe8000000000);

// pml4[510]
pub const KERNEL_HEAP_START: VirtAddr = VirtAddr::new(0xffffff0000000000);

const HDDM_PML4_INDEX: u64 = 508;
const KERNEL_THREAD_STACKS_PML4_INDEX: u64 = 509;
const KERNEL_HEAP_PML4_INDEX: u64 = 510;
const KERNEL_PML4_INDEX: u64 = 511;

pub const PAGE_ENTRIES: u64 = 512;

pub const PAGE_SIZE_4KIB: u64 = 4096;
pub const PAGE_SIZE_2MIB: u64 = PAGE_SIZE_4KIB * 512;

pub static HHDM_START: RwLock<VirtAddr> = RwLock::new(VirtAddr::zero());

// TODO: support other arches, and abstract all virtual memory operations
// TODO: remove locking every time
pub struct VirtualMemoryManager {
    initialized: bool,
}

impl VirtualMemoryManager {
    // Initializes the virtual memory manager
    pub fn init(&mut self, hhdm: VirtAddr) {
        let mut hhdm_start = HHDM_START.write();
        *hhdm_start = hhdm;

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

            self.map_pml2(pml2_table_phys, pml2_index, phys, pml2_flags);
        } else {
            let (pml2_flags, pml2_index) = (flags.to_plm2_flags(), virt.pml2_index());
            let pml1_table_phys = self.get_or_map_pml2(pml2_table_phys, pml2_index, pml2_flags);

            let (pml1_flags, pml1_index) = (flags.to_plm1_flags(), virt.pml1_index());
            self.map_pml1(pml1_table_phys, pml1_index, phys, pml1_flags);
        }

        flush_tlb_page(virt.get());

        if cfg!(vmm_debug) {
            println!(
                "VMM: mapped Virt {} -> Phys {} with flags {:?}",
                virt, phys, flags
            );
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

        flush_tlb_page(virt.get());

        if cfg!(vmm_debug) {
            println!("VMM: unmapped Virt {}", virt);
        }
    }

    pub fn get_page_entry_from_virt(
        &self,
        pml4_phys: PhysAddr,
        virt: VirtAddr,
    ) -> Option<(PhysAddr, PageFlags)> {
        let pml4_idx = virt.pml4_index();
        let pml3_idx = virt.pml3_index();
        let pml2_idx = virt.pml2_index();
        let pml1_idx = virt.pml1_index();

        let offset = virt.get() % 4096;

        let pml4 = self.get_pml4(pml4_phys, pml4_idx)?;
        let pml3 = self.get_pml3(pml4.0, pml3_idx)?;
        let pml2 = self.get_pml2(pml3.0, pml2_idx)?;
        let pml1 = self.get_pml1(pml2.0, pml1_idx)?;

        Some((
            PhysAddr::new((pml1.0.get() & 0xfffffffffffff000) + offset),
            PageFlags::from_bits(pml1.1.bits()).unwrap(),
        ))
    }

    fn map_physical_address_space(&self) {
        const PAGES_TO_MAP: u64 = PAGE_ENTRIES * PAGE_ENTRIES;

        let pml4_index = HDDM_VIRT_START.pml4_index();
        let pml4 = self.get_or_map_pml4(
            get_current_pml4(),
            pml4_index,
            PML4Flags::READ_WRITE | PML4Flags::PRESENT,
        );

        for pml3_index in 0..PAGE_ENTRIES {
            let pml3 =
                self.get_or_map_pml3(pml4, pml3_index, PML3Flags::READ_WRITE | PML3Flags::PRESENT);

            for pml2_index in 0..PAGE_ENTRIES {
                let phys_addr =
                    PhysAddr::new((pml3_index * PAGE_ENTRIES + pml2_index) * PAGE_SIZE_2MIB);

                self.map_pml2(
                    pml3,
                    pml2_index,
                    phys_addr,
                    PML2Flags::READ_WRITE | PML2Flags::PRESENT | PML2Flags::PAGE_SIZE,
                );
            }
        }

        let mut hddm_start = HHDM_START.write();
        let hddm_stack_diff = (HDDM_VIRT_START - *hddm_start).get();
        unsafe {
            hddm_adjust_offset = hddm_stack_diff;
        }

        *hddm_start = HDDM_VIRT_START;
    }

    pub fn map_4kib(&self, pml4: PhysAddr, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
        self.map(pml4, virt, phys, flags, false);
    }

    /// Maps a 2MiB page in memory
    pub fn map_2mib(&self, pml4: PhysAddr, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
        self.map(pml4, virt, phys, flags, true);
    }
}

pub static VIRTUAL_MEMORY_MANAGER: Mutex<VirtualMemoryManager> =
    Mutex::new(VirtualMemoryManager { initialized: false });

extern "C" {
    static __kernel_start: u64;
    static __kernel_end: u64;
    static mut hddm_adjust_offset: u64;
}

pub fn init(hhdm: u64) {
    let mut vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.init(VirtAddr(hhdm));
}

/// This function maps 512GiB into memory
pub fn map_physical_address_space() {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    vmm.map_physical_address_space();
}

pub fn unmap_limine_pages() {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    let pml4 = get_current_pml4();
    vmm.map_pml4(pml4, 0, PhysAddr::zero(), PML4Flags::NONE);
    vmm.map_pml4(pml4, 1, PhysAddr::zero(), PML4Flags::NONE);
    vmm.map_pml4(pml4, 256, PhysAddr::zero(), PML4Flags::NONE);
    vmm.map_pml4(pml4, 257, PhysAddr::zero(), PML4Flags::NONE);
}

pub fn dump_pml4(pml4_phys: PhysAddr) {
    let pml4 = pml4_phys.virt_addr().get() as *mut u64;
    for i in 0..PAGE_ENTRIES {
        let ent = unsafe { pml4.offset(i as isize).read() };
        if ent == 0 {
            continue;
        }
        println!("{}: {:#x}", i, ent);
    }
}

pub fn switch_pml4(pml4: PhysAddr) {
    unsafe { set_cr3(pml4.get()) };
}

pub fn copy_pml4_higher_half_entries(to: PhysAddr, from: PhysAddr) {
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();

    let pml4 = unsafe {
        slice::from_raw_parts_mut(to.virt_addr().get() as *mut u64, PAGE_ENTRIES as usize)
    };

    pml4[HDDM_PML4_INDEX as usize] = {
        let ent = vmm.get_pml4(from, HDDM_PML4_INDEX).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[KERNEL_THREAD_STACKS_PML4_INDEX as usize] = {
        let ent = vmm.get_pml4(from, KERNEL_THREAD_STACKS_PML4_INDEX).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[KERNEL_HEAP_PML4_INDEX as usize] = {
        let ent = vmm.get_pml4(from, KERNEL_HEAP_PML4_INDEX).unwrap();
        ent.0.get() | ent.1.bits()
    };

    pml4[KERNEL_PML4_INDEX as usize] = {
        let ent = vmm.get_pml4(from, KERNEL_PML4_INDEX).unwrap();
        ent.0.get() | ent.1.bits()
    };
}
