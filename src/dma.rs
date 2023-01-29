use spin::Mutex;

use crate::{
    arch::x86_64::paging::PageFlags,
    mm::{phys, virt, PhysAddr, VirtAddr},
};

/// pml4[507]
const DMA_START: VirtAddr = VirtAddr::new(0xfffffd8000000000);
static CURRENT_POINTER: Mutex<VirtAddr> = Mutex::new(DMA_START);

// FIXME: implement a better way to allocate dma regions
pub fn alloc(size: usize, phys_align: usize) -> (PhysAddr, VirtAddr) {
    let mut pointer = CURRENT_POINTER.lock();

    let in_pages = size / 4096;
    assert!(size % 4096 == 0);

    let phys = phys::alloc_multiple_align(in_pages, phys_align);
    let virt = *pointer;

    *pointer = virt + VirtAddr::new(size as u64);

    for i in 0..in_pages {
        let v = virt + VirtAddr::new(i as u64 * 4096);
        let p = phys + PhysAddr::new(i as u64 * 4096);
        virt::map(v, p, PageFlags::READ_WRITE);
    }

    (phys, virt)
}
