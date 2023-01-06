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

    pub fn alloc(&mut self) -> PhysAddr {
        for seg_idx in 0..self.segment_count {
            let local_bitmap_idx = match self.find_free_bitmap(seg_idx) {
                Some(x) => x,
                None => continue,
            };

            let segment = self.segments[seg_idx];
            let global_bitmap_idx = segment.global_bitmap_base + local_bitmap_idx;

            for bitmap_off in segment.lowest_idx..FRAMES_PER_BITMAP {
                // if the frame at bitmap_off is set then keep searching
                if self.bitmap[global_bitmap_idx] & (1 << bitmap_off) > 0 {
                    continue;
                }

                // mark the frame as allocated
                self.bitmap[global_bitmap_idx] |= 1 << bitmap_off;

                let local_frame_idx = local_bitmap_idx * FRAMES_PER_BITMAP + bitmap_off;
                let addr = PhysAddr::new((segment.base + local_frame_idx * FRAME_SIZE) as u64);

                if cfg!(pfa_debug) {
                    println!(
                        "PFA: allocated physical page {} segment: {} local index: {}",
                        addr, seg_idx, local_frame_idx
                    );
                }

                return addr;
            }
        }

        panic!("OUT OF MEMORY\n");
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

pub fn alloc() -> PhysAddr {
    let mut allocator = PHYS_ALLOCATOR.lock();
    allocator.alloc()
}
