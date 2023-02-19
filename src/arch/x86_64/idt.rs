const IDT_ENTRIES: usize = 256;

use super::{
    exception::*,
    gdt::{segment_selector, GDT_KERNEL_CODE},
};

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IDTEntry {
    offset_low: u16,
    segment_selector: u16,
    ist: u8, // 0..2 is the IST, rest is zero
    type_attributes: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

bitflags::bitflags! {
    pub struct IDTTypeAttr: u8 {
        const INTERRUPT_GATE = 0b1110;
        const TRAP_GATE = 0b1111;
        const RING0 = 0b00 << 5;
        const RING1 = 0b01 << 5;
        const RING2 = 0b10 << 5;
        const RING3 = 0b11 << 5;
        const PRESENT = 1 << 7;
    }
}

impl IDTEntry {
    pub const fn zero() -> IDTEntry {
        IDTEntry {
            offset_low: 0,
            segment_selector: 0,
            ist: 0,
            type_attributes: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub fn new(offset: u64, segment: u64, ist: u8, types: IDTTypeAttr) -> IDTEntry {
        IDTEntry {
            // TODO: check for valid input?
            offset_low: (offset & 0xFFFF) as u16,
            segment_selector: segment as u16,
            ist: ist,
            type_attributes: types.bits,
            offset_mid: ((offset >> 16) & 0xFFFF) as u16,
            offset_high: (offset >> 32) as u32,
            reserved: 0,
        }
    }
}

static mut IDT: [IDTEntry; IDT_ENTRIES] = [IDTEntry::zero(); IDT_ENTRIES];

#[repr(C, packed)]
struct IDTRValue {
    size: u16,
    addr: u64,
}

#[inline(always)]
unsafe fn load_idt(idt_descriptor: &IDTRValue) {
    core::arch::asm!("lidt [{}]", in(reg) idt_descriptor, options(nostack));
}

pub fn init() {
    // TODO: consider moving this somewhere else
    let exception_handlers: [u64; 32] = [
        __excp_div_by_zero as u64,                 // 0
        __excp_debug as u64,                       // 1
        __excp_non_maskable_interrutpt as u64,     // 2
        __excp_breakpoint as u64,                  // 3
        __excp_overflow as u64,                    // 4
        __excp_bound_range_exceeded as u64,        // 5
        __excp_invalid_opcode as u64,              // 6
        __excp_device_not_available as u64,        // 7
        __excp_double_fault as u64,                // 8
        __excp_coprocessor_segment_overrun as u64, // 9
        __excp_invalid_tss as u64,                 // 10
        __excp_segment_not_present as u64,         // 11
        __excp_stack_segment_fault as u64,         // 12
        __excp_general_protection_fault as u64,    // 13
        __excp_page_fault as u64,                  // 14
        0,                                         // 15 - reserved
        __excp_x87 as u64,                         // 16
        __excp_alignment_check as u64,             // 17
        __excp_machine_check as u64,               // 18
        __excp_simd_fpe as u64,                    // 19
        __excp_virtualization as u64,              // 20
        __excp_control_protection as u64,          // 21
        0,                                         // 22 - reserved
        0,                                         // 23 - reserved
        0,                                         // 24 - reserved
        0,                                         // 25 - reserved
        0,                                         // 26 - reserved
        0,                                         // 27 - reserved
        __excp_hypervisor_injection as u64,        // 28
        __excp_vmm_communication as u64,           // 29
        __excp_security as u64,                    // 30
        0,                                         // 31
    ];

    // cant make this as a constant unfortunately
    let kernel_code_type: IDTTypeAttr =
        IDTTypeAttr::TRAP_GATE | IDTTypeAttr::PRESENT | IDTTypeAttr::RING0;

    unsafe {
        for (i, addr) in exception_handlers.iter().enumerate() {
            IDT[i] = IDTEntry::new(
                *addr,
                segment_selector(GDT_KERNEL_CODE, 0),
                0,
                kernel_code_type,
            );
        }

        let idtr = IDTRValue {
            addr: IDT.as_ptr() as u64,
            size: (IDT_ENTRIES * core::mem::size_of::<IDTEntry>() - 1) as u16,
        };

        load_idt(&idtr);
    }
}

pub fn install_interrupt_handler(idx: usize, handler: u64, desc_type: IDTTypeAttr, privilege: u64) {
    assert!(idx < 256);
    assert!(privilege < 4);

    let selector = segment_selector(GDT_KERNEL_CODE, privilege);

    unsafe {
        IDT[idx] = IDTEntry::new(handler, selector, 0, desc_type);
    }
}
