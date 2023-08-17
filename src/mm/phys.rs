use alloc::vec::Vec;
use limine::{MemmapResponse, MemoryMapEntryType};

use spin::Mutex;

use crate::mm::PhysAddr;

const MAX_SEGMENT_COUNT: usize = 16;
pub const FRAME_SIZE: usize = 4096;
// 16 GiB
const MAX_FRAMES: usize = (16 * 1024 * 1024 * 1024) / FRAME_SIZE;
const FRAMES_PER_BITMAP: usize = core::mem::size_of::<usize>() * 8;
const BITMAP_SIZE: usize = MAX_FRAMES / FRAMES_PER_BITMAP;

// TODO: locking?
pub struct PageDescriptor {
    used_count: usize,
}

impl PageDescriptor {
    fn new() -> PageDescriptor {
        PageDescriptor { used_count: 0 }
    }
}

#[derive(Clone, Copy)]
struct PhysSegment {
    base: usize,
    len: usize, // in frames
    global_bitmap_base: usize,
    lowest_idx: usize,
}

impl PhysSegment {
    pub const fn new() -> PhysSegment {
        PhysSegment {
            base: 0,
            len: 0,
            global_bitmap_base: 0,
            lowest_idx: 0,
        }
    }
}

pub struct PageDescriptorManager {
    pub initialized: bool,
    page_descriptors: Vec<PageDescriptor>,
}

macro_rules! get_page_desc {
    ($self: ident, $addr: expr) => {{
        let idx = Self::phys_addr_to_index($addr);
        $self.page_descriptors.get(idx).unwrap()
    }};
}

macro_rules! get_page_desc_mut {
    ($self: ident, $addr: expr) => {{
        let idx = Self::phys_addr_to_index($addr);
        $self.page_descriptors.get_mut(idx).unwrap()
    }};
}

// TODO: atomic page descriptor?
impl PageDescriptorManager {
    fn phys_addr_to_index(addr: PhysAddr) -> usize {
        assert!(addr.is_aligned());
        addr.0 as usize / FRAME_SIZE
    }

    fn init(&mut self, frame_count: usize) {
        self.initialized = true;
        self.page_descriptors
            .resize_with(frame_count, PageDescriptor::new);
    }

    pub fn inc_used_count(&mut self, addr: PhysAddr) {
        let page_desc = get_page_desc_mut!(self, addr);
        page_desc.used_count += 1;
    }

    pub fn dec_used_count(&mut self, addr: PhysAddr) {
        let page_desc = get_page_desc_mut!(self, addr);
        if page_desc.used_count > 1 {
            page_desc.used_count -= 1;
        } else {
            warn!("used_count is 0 but we are trying to decrement it");
        }

        if page_desc.used_count == 0 {
            // TODO: free frame
        }
    }

    fn get_used_count(&self, addr: PhysAddr) -> usize {
        let page_desc = get_page_desc!(self, addr);
        page_desc.used_count
    }
}

pub static PAGE_DESCRIPTOR_MANAGER: Mutex<PageDescriptorManager> =
    Mutex::new(PageDescriptorManager {
        initialized: false,
        page_descriptors: Vec::new(),
    });

pub struct PhysAllocator {
    segments: [PhysSegment; MAX_SEGMENT_COUNT],
    segment_count: usize,
    bitmap: [usize; BITMAP_SIZE],
    total_frames: usize,
    used_frames: usize,
}

impl PhysAllocator {
    pub fn init(&mut self, memmap: &MemmapResponse) {
        let mut bitmap_base: usize = 0;
        let mmap = memmap.entries.as_ptr();
        for i in 0..memmap.entry_count {
            let entry = unsafe {
                // TODO: im not sure if theres a better way to do this
                mmap.offset(i as isize)
                    .as_ref()
                    .expect("invalid memory map response")
            };

            if entry.typ != MemoryMapEntryType::Usable {
                continue;
            }

            assert!(entry.base % FRAME_SIZE as u64 == 0);
            let frames = (entry.len / FRAME_SIZE as u64) as usize;
            self.segments[self.segment_count] = PhysSegment {
                base: entry.base as usize,
                len: frames,
                global_bitmap_base: bitmap_base,
                lowest_idx: 0,
            };

            self.segment_count += 1;
            self.total_frames += frames;

            bitmap_base += frames / FRAMES_PER_BITMAP;
            let rem_frames = frames % FRAMES_PER_BITMAP;
            // sometimes the last isn't filled completely, so we mark the
            // unusable bits as allocated
            if rem_frames != 0 {
                self.bitmap[bitmap_base] = usize::MAX << rem_frames;
                bitmap_base += 1;
            }
        }
        self.used_frames = self.total_frames;

        self.print_available_memory();
    }

    pub fn init_page_descriptors(&mut self) {
        let last_seg = &self.segments[self.segment_count - 1];
        let last_frame_addr = last_seg.base + last_seg.len * FRAME_SIZE;

        let frame_count = last_frame_addr / FRAME_SIZE;
        let size = frame_count * core::mem::size_of::<PageDescriptor>();

        // FIXME: if the requested size is bigger than kalloc's initial heap size
        // the kernel hangs
        let mut pgm = PAGE_DESCRIPTOR_MANAGER.lock();
        pgm.init(frame_count);

        log!("{} bytes allocated for {} frames", size, frame_count);

        // TODO: set currently used frames
    }

    fn print_available_memory(&self) {
        for i in 0..self.segment_count {
            let segment = self.segments[i];
            log!(
                "segment {}: {:#x} {} pages bitmap base: {}",
                i,
                segment.base,
                segment.len,
                segment.global_bitmap_base
            );
        }

        let mut kib = (self.total_frames * FRAME_SIZE) / 1024;
        let mib = kib / 1024;
        kib -= mib * 1024;
        log!("available system memory: {} MiB {} KiB", mib, kib);
    }

    // find a free bitmap in segment
    // returns the local index
    fn find_free_bitmap(&self, segment_idx: usize) -> Option<usize> {
        let segment = self.segments[segment_idx];

        // calculate how many frames are in a single bitmap,
        // on 32bit this is 32
        // on 64bit this is 64
        let bitmap_rem = segment.len % FRAMES_PER_BITMAP;
        let bitmap_count = if bitmap_rem == 0 {
            segment.len / FRAMES_PER_BITMAP
        } else {
            segment.len / FRAMES_PER_BITMAP + 1
        };

        for bitmap_idx in 0..bitmap_count {
            let global_bitmap_idx = segment.global_bitmap_base + bitmap_idx;
            let bitmap = self.bitmap[global_bitmap_idx];

            // if all the frames in the bitmap are set continue
            if bitmap == usize::MAX {
                continue;
            }

            return Some(bitmap_idx);
        }

        None
    }

    fn calculate_addr(&self, segment_idx: usize, idx: usize) -> PhysAddr {
        let segment = self.segments[segment_idx];
        PhysAddr::new((segment.base + idx * FRAME_SIZE) as u64)
    }

    fn segment_find_region(&self, segment_idx: usize, size: usize, align: usize) -> Option<usize> {
        let mut current_count = 0;
        let mut current_start = 0;

        let segment = self.segments[segment_idx];

        let rem = segment.base % align;
        let start_off_to_align = if rem == 0 { 0 } else { align - rem };
        let start_off_in_pages = start_off_to_align / FRAME_SIZE;

        let mut bitmaps = segment.len / FRAMES_PER_BITMAP;
        if bitmaps > 0 {
            bitmaps += 1;
        }

        let mut step = size / FRAMES_PER_BITMAP;
        if size % FRAMES_PER_BITMAP != 0 {
            step += 1;
        }

        let page_align = align >> 12;

        'bm_loop: for bitmap_idx in (start_off_in_pages..bitmaps).step_by(step) {
            let left = segment.len - bitmap_idx * FRAMES_PER_BITMAP;
            let bits = usize::min(FRAMES_PER_BITMAP, left);
            for bitmap_off in 0..bits {
                let global_bitmap_idx = segment.global_bitmap_base + bitmap_idx;
                // if the frame at bitmap_off is set then keep searching
                if self.bitmap[global_bitmap_idx] & (1 << bitmap_off) > 0 {
                    current_count = 0;
                    continue;
                }

                if current_count == 0 {
                    current_start = bitmap_idx * FRAMES_PER_BITMAP + bitmap_off;
                    if current_start % page_align != 0 {
                        continue 'bm_loop;
                    }
                }

                current_count += 1;

                if current_count == size {
                    return Some(current_start);
                }
            }
        }
        None
    }

    /// Returns a segment and a corresponding local bitmap index that satisfies
    /// the size and alignment parameters
    /// Returns  None if no such region was found
    fn find_region(&self, size: usize, align: usize) -> Option<(usize, usize)> {
        for seg_idx in 0..self.segment_count {
            let ret = self.segment_find_region(seg_idx, size, align);
            if let Some(bitmap_idx) = ret {
                return Some((seg_idx, bitmap_idx));
            }
        }

        None
    }

    /// Marks the specified region in the segment as allocated, no checks are performed
    fn mark_region_as_allocated(&mut self, segment_idx: usize, start_idx: usize, size: usize) {
        let segment = self.segments[segment_idx];

        let mut size_left = size;
        let mut bitmap_idx = segment.global_bitmap_base + start_idx / FRAMES_PER_BITMAP;
        let mut bitmap_off = start_idx % FRAMES_PER_BITMAP;

        while size_left > 0 {
            if bitmap_off == 0 && size_left >= FRAMES_PER_BITMAP {
                self.bitmap[bitmap_idx] = usize::MAX;

                bitmap_idx += 1;
                size_left -= FRAMES_PER_BITMAP;
                continue;
            } else if size_left < FRAMES_PER_BITMAP {
                let size = usize::MAX >> (FRAMES_PER_BITMAP - size_left);
                self.bitmap[bitmap_idx] |= size << bitmap_off;

                return;
            } else {
                self.bitmap[bitmap_idx] |= usize::MAX << bitmap_off;

                size_left = FRAMES_PER_BITMAP - bitmap_off;
                bitmap_idx += 1;
                bitmap_off = 0;
            }
        }
    }

    fn alloc_multiple(&mut self, size: usize, align: usize) -> PhysAddr {
        assert!(align % 4096 == 0);

        let region = self.find_region(size, align);
        if region.is_none() {
            panic!("OUT OF MEMORY");
        }

        let region = region.unwrap();

        self.mark_region_as_allocated(region.0, region.1, size);

        let addr = self.calculate_addr(region.0, region.1);
        if cfg!(pfa_debug) {
            log!(
                "PFA: allocated {} physical pages at {} align: {} segment: {} local index: {}",
                size,
                addr,
                align,
                region.0,
                region.1
            );
        }

        addr
    }

    pub const fn new_uninit() -> PhysAllocator {
        PhysAllocator {
            segments: [PhysSegment::new(); MAX_SEGMENT_COUNT],
            segment_count: 0,
            bitmap: [0; BITMAP_SIZE],
            total_frames: 0,
            used_frames: 0,
        }
    }
}

pub static PHYS_ALLOCATOR: Mutex<PhysAllocator> = Mutex::new(PhysAllocator::new_uninit());

pub fn init(memmap: &MemmapResponse) {
    let mut allocator = PHYS_ALLOCATOR.lock();
    allocator.init(memmap);
}

pub fn init_page_descriptors() {
    let mut allocator = PHYS_ALLOCATOR.lock();
    allocator.init_page_descriptors();
}

pub fn alloc_multiple_align(size: usize, align: usize) -> PhysAddr {
    let mut allocator = PHYS_ALLOCATOR.lock();
    allocator.alloc_multiple(size, align)
}

pub fn alloc_multiple(size: usize) -> PhysAddr {
    alloc_multiple_align(size, 0x1000)
}

/// Allocates a single page
pub fn alloc() -> PhysAddr {
    alloc_multiple(1)
}
