use limine::{LimineMemmapResponse, LimineMemoryMapEntryType};

use spin::Mutex;

use crate::mm::PhysAddr;

const MAX_SEGMENT_COUNT: usize = 16;
// 16 GiB
const FRAME_SIZE: usize = 4096;
const MAX_FRAMES: usize = (16 * 1024 * 1024 * 1024) / FRAME_SIZE;
const FRAMES_PER_BITMAP: usize = core::mem::size_of::<usize>() * 8;
const BITMAP_SIZE: usize = MAX_FRAMES / FRAMES_PER_BITMAP;

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

struct PhysAllocator {
    segments: [PhysSegment; MAX_SEGMENT_COUNT],
    segment_count: usize,
    bitmap: [usize; BITMAP_SIZE],
    total_frames: usize,
    used_frames: usize,
}

impl PhysAllocator {
    pub fn init(&mut self, memmap: &LimineMemmapResponse) {
        let mut bitmap_base: usize = 0;
        for i in 0..memmap.entry_count {
            let entry = unsafe {
                // TODO: im not sure if theres a better way to do this
                memmap
                    .entries
                    .as_ptr()
                    .offset(i as isize)
                    .as_ref()
                    .expect("invalid memory map response")
                    .as_ptr()
                    .as_ref()
                    .expect("invalid memory map response")
            };
            if entry.typ != LimineMemoryMapEntryType::Usable {
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

    fn print_available_memory(&self) {
        for i in 0..self.segment_count {
            let segment = self.segments[i];
            println!(
                "segment {}: {:#x} {} pages bitmap base: {}",
                i, segment.base, segment.len, segment.global_bitmap_base
            );
        }

        let mut kib = (self.total_frames * FRAME_SIZE) / 1024;
        let mib = kib / 1024;
        kib -= mib * 1024;
        println!("available system memory: {} MiB {} KiB", mib, kib);
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
            } else {
                if size_left < FRAMES_PER_BITMAP {
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
    }

    pub fn alloc_multiple(&mut self, size: usize, align: usize) -> PhysAddr {
        assert!(align % 4096 == 0);

        let region = self.find_region(size, align);
        if region.is_none() {
            panic!("OUT OF MEMORY");
        }

        let region = region.unwrap();

        self.mark_region_as_allocated(region.0, region.1, size);

        let addr = self.calculate_addr(region.0, region.1);
        if cfg!(pfa_debug) {
            println!(
                "PFA: allocated {} physical pages at {} align: {} segment: {} local index: {}",
                size, addr, align, region.0, region.1
            );
        }

        addr
    }

    pub const fn new() -> PhysAllocator {
        PhysAllocator {
            segments: [PhysSegment::new(); MAX_SEGMENT_COUNT],
            segment_count: 0,
            bitmap: [0; BITMAP_SIZE],
            total_frames: 0,
            used_frames: 0,
        }
    }
}

static PHYS_ALLOCATOR: Mutex<PhysAllocator> = Mutex::new(PhysAllocator::new());

pub fn init(memmap: &LimineMemmapResponse) {
    let mut allocator = PHYS_ALLOCATOR.lock();
    allocator.init(memmap);
}

/// Allocates multiple contiguous pages
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
