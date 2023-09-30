use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;

use crate::{
    arch::x86_64::{get_current_pml4, paging::PageFlags},
    utils,
};

use super::{
    virt::{KERNEL_HEAP_START, PML4},
    VirtAddr,
};

const KERNEL_HEAP_BASE_SIZE: usize = 1024 * 1024; // 1024 KiB
const MINIMUM_REGION_SIZE: usize = 8;

#[derive(Clone, Copy)]
struct Node {
    size: usize,
    allocated: bool,
}

struct KernelAllocator;

struct KernelAllocatorInner {
    current_size: usize,
    allocated_nodes: usize,
    initialized: bool,
}

impl Node {
    fn next(&self) -> Option<&mut Node> {
        assert_ne!(self.size, 0);
        // FIXME: handle out of memory
        let ptr =
            (self as *const _ as usize + core::mem::size_of::<Node>() + self.size) as *mut Node;
        Some(unsafe { ptr.as_mut().unwrap() })
    }
}

unsafe impl Send for Node {}
unsafe impl Send for KernelAllocatorInner {}

#[global_allocator]
static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator;
static KERNEL_ALLOCATOR_INNER: Mutex<KernelAllocatorInner> = Mutex::new(KernelAllocatorInner {
    current_size: 0,
    allocated_nodes: 0,
    initialized: false, // FIXME: this ^^
});

impl KernelAllocatorInner {
    fn head() -> &'static mut Node {
        unsafe { (KERNEL_HEAP_START.get() as *mut Node).as_mut().unwrap() }
    }

    fn heap_end(&self) -> VirtAddr {
        VirtAddr::new(KERNEL_HEAP_START.get() + self.current_size as u64)
    }

    fn extend_heap(&mut self, min_size: usize) -> usize {
        let pml4 = get_current_pml4();

        let mut size = self.current_size;
        while size < min_size {
            size *= 2;
        }

        let newly_allocated_size = size - self.current_size;

        debug!("{} {} {} {}", newly_allocated_size, size, min_size, self.current_size);

        let start_virt = self.heap_end();
        let end_virt = self.heap_end() + VirtAddr::new(newly_allocated_size as u64);
        let flags = PageFlags::READ_WRITE | PageFlags::PRESENT;

        pml4.map_range(start_virt, end_virt, flags);

        newly_allocated_size
    }

    ///
    fn get_free_region(&mut self, size: usize, align: usize) -> Option<usize> {
        const MIN_SIZE: usize = core::mem::size_of::<Node>() + MINIMUM_REGION_SIZE;

        // ensure that headers are aligned to pointer size boundaries(4 on 32bit, 8 on 64bit...)
        let size = utils::align(size, core::mem::size_of::<usize>());

        let mut current = KernelAllocatorInner::head();
        let mut has_next = true;

        while has_next {
            let heap_end = self.heap_end();
            let current_addr = current as *const _ as u64;
            assert!(heap_end.get() >= current_addr);
            // extend heap when we reach the end of the heap
            if heap_end.get() == current_addr {
                let extended = self.extend_heap(self.current_size + size);
                current.size = extended - core::mem::size_of::<Node>();
                current.allocated = false;
            }

            assert_ne!(current.size, 0);
            if current.allocated || current.size < size {
                let next = current.next();
                has_next = next.is_some();
                current = next.unwrap();
                continue;
            }

            let header_addr = current as *const _ as usize;

            let region_start = header_addr + core::mem::size_of::<Node>();
            let region_end = header_addr + current.size;

            let actual_region_start = utils::align(region_start, align);

            let right_side = current.size > (size + MIN_SIZE);
            let split_prev = actual_region_start != region_start;

            return if split_prev || right_side {
                let aligned_size = region_end - actual_region_start;

                // add header to the size
                let total_size = core::mem::size_of::<Node>() + usize::min(aligned_size, size);
                let remaining_size = current.size - total_size;

                // check if the new region is suitable and the old region is big enough
                if aligned_size < size || remaining_size < MIN_SIZE {
                    let next = current.next();
                    has_next = next.is_some();
                    current = next.unwrap();
                    continue;
                }

                if right_side {
                    // the new header is after the current header
                    let header_addr = actual_region_start + size;
                    let new_node = unsafe { (header_addr as *mut Node).as_mut().unwrap() };
                    new_node.allocated = false;
                    new_node.size = remaining_size;

                    current.allocated = true;
                    current.size = size;
                } else {
                    // the new header is before the current header
                    current.size = remaining_size;

                    let header_addr = actual_region_start - core::mem::size_of::<Node>();
                    let new_node = unsafe { (header_addr as *mut Node).as_mut().unwrap() };
                    new_node.allocated = true;
                    new_node.size = size;
                }

                Some(actual_region_start)
            } else {
                if current.size < size {
                    let next = current.next();
                    has_next = next.is_some();
                    current = next.unwrap();
                    continue;
                }

                current.allocated = true;
                self.allocated_nodes += 1;
                Some(region_start)
            };
        }

        None
    }

    fn free_region(&mut self, addr: usize) {
        let header_addr = addr - core::mem::size_of::<Node>();
        let region = unsafe { (header_addr as *mut Node).as_mut().unwrap() };
        assert!(region.allocated);
        region.allocated = false;
    }

    pub fn init(&mut self, pml4: &PML4) {
        assert!(!self.initialized);

        self.initialized = true;
        self.current_size = KERNEL_HEAP_BASE_SIZE;

        let start_virt = KERNEL_HEAP_START;
        let end_virt = KERNEL_HEAP_START + VirtAddr::new(self.current_size as u64);
        let flags = PageFlags::READ_WRITE | PageFlags::PRESENT;

        pml4.map_range(start_virt, end_virt, flags);

        let head = KernelAllocatorInner::head();
        head.allocated = false;
        head.size = self.current_size - core::mem::size_of::<Node>();
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut inner = KERNEL_ALLOCATOR_INNER.lock();
        assert!(inner.initialized);

        let region = inner
            .get_free_region(layout.size(), layout.align())
            .expect("OUT OF MEMORY");

        region as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        let mut inner = KERNEL_ALLOCATOR_INNER.lock();
        assert!(inner.initialized);

        inner.free_region(ptr as usize);
    }
}

pub fn init(pml4: &PML4) {
    let mut data = KERNEL_ALLOCATOR_INNER.lock();
    data.init(pml4);
}
