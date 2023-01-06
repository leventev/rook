use core::{alloc::{GlobalAlloc, Layout}, ptr::{null_mut, null}};
use spin::Mutex;

use crate::arch::x86_64::paging::PageFlags;

use super::{VirtAddr, virt, phys};

const KERNEL_HEAP_START: VirtAddr = VirtAddr::new(0xfffffe8000000000);
const KERNEL_HEAP_BASE_SIZE: usize = 128 * 1024; // 128 KiB

struct KernelAllocator;

struct KernelAllocatorData {
    current_size: usize,
    allocated_nodes: usize,
    initialized: bool,
}

#[global_allocator]
static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator;
static KERNEL_ALLOCATOR_DATA: Mutex<KernelAllocatorData> = Mutex::new(KernelAllocatorData {
    current_size: 0,
    allocated_nodes: 0,
    initialized: false,
});

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut data = KERNEL_ALLOCATOR_DATA.lock();
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut data = KERNEL_ALLOCATOR_DATA.lock();
    }
}

impl KernelAllocatorData {
    pub fn init(&mut self) {
        assert!(!self.initialized);
        self.initialized = true;
        self.current_size = KERNEL_HEAP_BASE_SIZE;

        let pages = self.current_size / 4096;
        for i in 0..pages {
            let virt = KERNEL_HEAP_START + VirtAddr(i as u64 * 4096);
            let phys = phys::alloc();
            virt::map(virt, phys, PageFlags::READ_WRITE);
        }
    }
}

pub fn init() {
    println!("kalloc init");
    let mut data = KERNEL_ALLOCATOR_DATA.lock();
    data.init();
}
