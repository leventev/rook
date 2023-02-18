#[repr(C, packed)]
pub struct TaskStateSegment {
    pub __reserved_0: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    pub __reserved_1: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    pub __reserved_2: u64,
    pub __reserved_3: u16,
    pub io_map_base_addr: u16,
}

impl TaskStateSegment {
    const fn zero() -> TaskStateSegment {
        Self {
            __reserved_0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            __reserved_1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            __reserved_2: 0,
            __reserved_3: 0,
            io_map_base_addr: 0,
        }
    }
}

pub static mut TSS: TaskStateSegment = TaskStateSegment::zero();
