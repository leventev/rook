use super::tss::{TaskStateSegment, TSS};

// Only the necessary values are defined
const GDT_SEGMENT_READABLE: u8 = 1 << 1;
const GDT_SEGMENT_WRITEABLE: u8 = 1 << 1;
const GDT_SEGMENT_EXECUTABLE: u8 = 1 << 3;
const GDT_SEGMENT_USER: u8 = 1 << 4;
const GDT_SEGMENT_RING3: u8 = 3 << 5;
const GDT_SEGMENT_RING0: u8 = 0;
const GDT_SEGMENT_PRESENT: u8 = 1 << 7;
const GDT_TSS_SYSTEM_AVAILABE: u8 = 0b1001;

const GDT_SEGMENT_LONG_MODE: u8 = 1 << 5;

const GDT_KERNEL_CODE_FLAGS: u8 = GDT_SEGMENT_USER
    | GDT_SEGMENT_RING0
    | GDT_SEGMENT_EXECUTABLE
    | GDT_SEGMENT_READABLE
    | GDT_SEGMENT_PRESENT;

const GDT_KERNEL_DATA_FLAGS: u8 =
    GDT_SEGMENT_USER | GDT_SEGMENT_RING0 | GDT_SEGMENT_WRITEABLE | GDT_SEGMENT_PRESENT;

const GDT_USER_CODE_FLAGS: u8 = GDT_SEGMENT_USER
    | GDT_SEGMENT_RING3
    | GDT_SEGMENT_EXECUTABLE
    | GDT_SEGMENT_READABLE
    | GDT_SEGMENT_PRESENT;

const GDT_USER_DATA_FLAGS: u8 =
    GDT_SEGMENT_USER | GDT_SEGMENT_RING3 | GDT_SEGMENT_WRITEABLE | GDT_SEGMENT_PRESENT;

const GDT_TSS_FLAGS: u8 = GDT_SEGMENT_RING3 | GDT_TSS_SYSTEM_AVAILABE | GDT_SEGMENT_PRESENT;

#[repr(C, packed)]
pub struct GDTEntry {
    limit_1: u16,
    base_address_1: u16,
    base_address_2: u8,
    _type: u8,
    limit_2_and_flags: u8,
    base_address_3: u8,
}

impl GDTEntry {
    const fn new(base_addr: u32, limit: u32, flags: u8) -> GDTEntry {
        GDTEntry {
            limit_1: (limit & 0xFFFF) as u16,
            limit_2_and_flags: ((limit >> 16) & 0xF) as u8 | (GDT_SEGMENT_LONG_MODE) as u8,
            base_address_1: (base_addr & 0xFFFF) as u16,
            base_address_2: ((base_addr >> 16) & 0xFF) as u8,
            base_address_3: ((base_addr >> 24) & 0xFF) as u8,
            _type: flags,
        }
    }

    const fn null() -> GDTEntry {
        GDTEntry {
            limit_1: 0,
            base_address_1: 0,
            base_address_2: 0,
            _type: 0,
            limit_2_and_flags: 0,
            base_address_3: 0,
        }
    }
}

pub const GDT_NULL: u64 = 0x0;
pub const GDT_KERNEL_CODE: u64 = 1 * 0x8;
pub const GDT_KERNEL_DATA: u64 = 2 * 0x8;
pub const GDT_USER_CODE: u64 = 3 * 0x8;
pub const GDT_USER_DATA: u64 = 4 * 0x8;
pub const GDT_TSS_LOW: u64 = 5 * 0x8;
pub const GDT_TSS_HIGH: u64 = 6 * 0x8;

static mut GDT: [GDTEntry; 7] = [
    GDTEntry::null(),
    GDTEntry::new(0x0, 0xffffffff, GDT_KERNEL_CODE_FLAGS),
    GDTEntry::new(0x0, 0xffffffff, GDT_KERNEL_DATA_FLAGS),
    GDTEntry::new(0x0, 0xffffffff, GDT_USER_CODE_FLAGS),
    GDTEntry::new(0x0, 0xffffffff, GDT_USER_DATA_FLAGS),
    GDTEntry::null(), // TSS low
    GDTEntry::null(), // TSS high
];

pub const fn segment_selector(segment_idx: u64, priv_level: u64) -> u64 {
    assert!(segment_idx % 8 == 0);
    assert!(priv_level < 4);
    segment_idx | priv_level
}

#[repr(C, packed)]
pub struct GDTDescriptor {
    limit: u16,
    addr: u64,
}

#[no_mangle]
static mut GDT_DESCRIPTOR: GDTDescriptor = GDTDescriptor { limit: 0, addr: 0 };

extern "C" {
    fn load_gdt();
}

pub fn init() {
    unsafe {
        let tss_ptr = &TSS as *const _ as u64;
        let gdt_ptr = &GDT as *const _ as u64;

        GDT[5] = GDTEntry::new(
            tss_ptr as u32,
            core::mem::size_of::<TaskStateSegment>() as u32 - 1,
            GDT_TSS_FLAGS,
        );
        GDT[6] = GDTEntry::new((tss_ptr >> 48) as u32, ((tss_ptr >> 32) & 0xFFFF) as u32, 0);

        GDT_DESCRIPTOR.limit = (GDT.len() * core::mem::size_of::<GDTEntry>()) as u16 - 1;
        GDT_DESCRIPTOR.addr = gdt_ptr;

        load_gdt();
    }
}
