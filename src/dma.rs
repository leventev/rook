use crate::mm::{PhysAddr, VirtAddr};

//static CURRENT_POINTER: Mutex<VirtAddr> = Mutex::new(DMA_START);

// FIXME: implement a better way to allocate dma regions
pub fn alloc(_size: usize, _phys_align: usize) -> (PhysAddr, VirtAddr) {
    /*
    let mut pointer = CURRENT_POINTER.lock();

    let in_pages = size / 4096;
    assert!(size % 4096 == 0);

    let phys = phys::alloc_multiple_align(in_pages, phys_align);
    let virt = *pointer;

    *pointer = virt + VirtAddr::new(size as u64);

    for i in 0..in_pages {
        let v = virt + VirtAddr::new(i as u64 * 4096);
        let p = phys + PhysAddr::new(i as u64 * 4096);
        virt::map(v, p, PML1Flags::READ_WRITE);
    }

    (phys, virt)*/
    todo!()
}
