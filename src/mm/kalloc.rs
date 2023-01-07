use core::{
    alloc::{GlobalAlloc, Layout},
    mem,
    ptr::{null, null_mut},
};
use spin::Mutex;

use crate::arch::x86_64::paging::PageFlags;

use super::{phys, virt, VirtAddr};

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

impl KernelAllocator {
    fn align(n: usize, align_by: usize) -> usize {
        if n % align_by == 0 {
            n
        } else {
            n + (align_by - n % align_by)
        }
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut data = KERNEL_ALLOCATOR_DATA.lock();
        assert!(data.initialized);

        let mem_end = (KERNEL_HEAP_START.get() + data.current_size as u64) as *mut u64;
        let size = KernelAllocator::align(layout.size(), mem::size_of::<u64>());

        if cfg!(kalloc_debug) {
            println!(
                "KALLOC: trying to allocate {} bytes, aligned to {}",
                size,
                layout.align()
            );
        }

        // TODO: cache the starting position
        let mut header: *mut u64 = KERNEL_HEAP_START.get() as *mut u64;
        while header < mem_end {
            if *header == 0 {
                header = KernelAllocator::align(header as usize, layout.align()) as *mut u64;

                let size_left = mem_end as usize - header as usize - mem::size_of::<u64>();
                if size_left < size {
                    break;
                }

                *header = (1 << 63) | (size as u64);

                let next_header = header.offset(size as isize / mem::size_of::<u64>() as isize + 1);
                *next_header = 0;

                if cfg!(kalloc_debug) {
                    println!(
                        "KALLOC: found empty header at {:#?} next_header: {:#?}",
                        header, next_header
                    );
                }

                let chunk_start = header.offset(1) as *mut u8;

                if cfg!(kalloc_debug) {
                    println!(
                        "KALLOC: allocated chunk {:#?} with size {}",
                        chunk_start, size
                    );
                }

                data.allocated_nodes += 1;

                return chunk_start;
            }

            let mut chunk_size = *header & !(1 << 63);
            assert!(
                chunk_size as usize % mem::size_of::<u64>() == 0,
                "Invalid header size"
            );
            
            let present = (*header & (1 << 63)) != 0;
            let aligned_header =
            KernelAllocator::align(header as usize, layout.align()) as *mut u64;
            
            let advance = aligned_header as usize - header as usize;
            
            let prev_header_size = if advance > 0 {
                advance - mem::size_of::<u64>()
            } else {
                0
            };
            
            let chunk_size_old = chunk_size;
            chunk_size -= prev_header_size as u64;

            // the chunk is suitable and isnt present
            if chunk_size >= size as u64 && !present {
                // i couldnt test the alignment code, lets hope it works
                // alignment happened
                if advance > 0 {
                    //assert!(false);
                    // truncate the base header
                    assert!(prev_header_size >= mem::size_of::<u64>());
                    *header = prev_header_size as u64;
                    header = aligned_header;
                }

                let rem_size = chunk_size - size as u64;

                let final_size: u64;
                // we can split the chunk into two chunks
                if rem_size as usize >= 2 * mem::size_of::<u64>() {
                    let new_header = header.offset(size as isize / 4 + 1);
                    *new_header = rem_size - 4;
                    *header = (1 << 63) | size as u64;
                    final_size = size as u64;

                    if cfg!(kalloc_debug) {
                        println!(
                            "KALLOC: chunk({}) split into two {:#?}({}), {:#?}({})",
                            chunk_size, header, size, new_header, rem_size
                        );
                    }
                } else {
                    *header = (1 << 63) | chunk_size;
                    final_size = chunk_size;

                    if cfg!(kalloc_debug) {
                        println!(
                            "KALLOC: merged unsplittable chunk({}) into new chunk at {:#?}",
                            chunk_size,
                            header.offset(1)
                        );
                    }
                }

                let chunk_start = header.offset(1) as *mut u8;
                if cfg!(kalloc_debug) {
                    println!(
                        "KALLOC: allocated chunk {:#?} with size {}",
                        chunk_start, final_size
                    );
                }

                data.allocated_nodes += 1;
                return chunk_start;
            }

            // if the chunk wasnt suitable jump to the next header
            let jump = (chunk_size / mem::size_of::<u64>() as u64) + 1;
            let next_header = header.offset(jump as isize);
            if cfg!(kalloc_debug) {
                println!(
                    "KALLOC: jump {:#?} -> {:#?} {} bytes present: {} chunk_size: {}",
                    header,
                    next_header,
                    jump * mem::size_of::<u64>() as u64,
                    present,
                    chunk_size_old
                );
            }

            header = next_header;

            continue;
        }

        panic!("OUT OF MEMORY");
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut data = KERNEL_ALLOCATOR_DATA.lock();
        assert!(data.initialized);

        data.allocated_nodes -= 1;

        let header: *mut u64 = (ptr as *mut u64).offset(-1);
        *header &= !(1 << 63);
        if cfg!(kalloc_debug) {
            println!(
                "KALLOC: deallocated chunk {:#?} with size {}",
                header.offset(1),
                *header
            );
        }
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
    let mut data = KERNEL_ALLOCATOR_DATA.lock();
    data.init();
}
