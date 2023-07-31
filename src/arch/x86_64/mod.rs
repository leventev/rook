pub mod exception;
pub mod gdt;
pub mod idt;
pub mod paging;
pub mod pic;
pub mod registers;
pub mod stacktrace;
pub mod syscall;
pub mod tss;

use core::arch::asm;

use crate::mm::{virt::PML4, PhysAddr};

bitflags::bitflags! {
    pub struct Rflags: u64 {
        const CARRY = 1 << 0;
        const RESERVED_BIT_1 = 1 << 1;
        const PARITY = 1 << 2;
        const AUXILIARY_CARRY = 1 << 4;
        const ZERO = 1 << 6;
        const SIGN = 1 << 7;
        const TRAP = 1 << 8;
        const INTERRUPT = 1 << 9;
        const DIRECTION = 1 << 10;
        const OVERFLOW = 1 << 11;
        const NESTED_TASK = 1 << 14;
        const RESUME = 1 << 16;
        const VIRTUAL8086 = 1 << 17;
        const ALIGNMENT_CHECK = 1 << 18;
        const VIRTUAL_INTERRUPT = 1 << 19;
        const VIRTUAL_INTERRUPT_PENDING = 1 << 20;
        const ID = 1 << 21;

        const THREAD_DEFAULT = Self::INTERRUPT.bits() | Self::RESERVED_BIT_1.bits();
    }

    pub struct CR0Flags: u64 {
        const PE = 1 << 0;
        const MP = 1 << 1;
        const EM = 1 << 2;
        const TS = 1 << 3;
        const ET = 1 << 4;
        const NE = 1 << 5;
        const WP = 1 << 16;
        const AM = 1 << 18;
        const NW = 1 << 29;
        const CD = 1 << 30;
        const PG = 1 << 31;
    }

    pub struct CR4Flags: u64 {
        const VME = 1 << 0;
        const PVI = 1 << 1;
        const TSD = 1 << 2;
        const DE = 1 << 3;
        const PSE = 1 << 4;
        const PAE = 1 << 5;
        const MCE = 1 << 6;
        const PGE = 1 << 7;
        const PCE = 1 << 8;
        const OSFXSR = 1 << 9;
        const OSXMMEXCPT = 1 << 10;
        const UMIP = 1 << 11;
        const VMXE = 1 << 13;
        const SMXE = 1 << 14;
        const FSGSBASE = 1 << 16;
        const PCIDE = 1 << 17;
        const OSXSAVE = 1 << 18;
        const SMEP = 1 << 20;
        const SMAP = 1 << 21;
        const PKE = 1 << 22;
        const CET = 1 << 23;
        const PKS = 1 << 24;
    }

    pub struct XCR0Flags: u64 {
        const X87 = 1 << 0;
        const SSE = 1 << 1;
        const AVX = 1 << 2;
        const BNDREG = 1 << 3;
        const BNDCSR = 1 << 4;
        const OPMASK = 1 << 5;
        const ZMM_HI256 = 1 << 6;
        const HI16_ZMM = 1 << 7;
        const PKRU = 1 << 9;
    }

    pub struct X87Flags: u64 {
        const EXCEPTION_INVALID_OPERATION = 1 << 0;
        const EXCEPTION_DENORMAL_OPERAND = 1 << 1;
        const EXCEPTION_DIVIDE_BY_ZERO = 1 << 2;
        const EXCEPTION_OVERFLOW = 1 << 3;
        const EXCEPTION_UNDERFLOW = 1 << 4;
        const EXCEPTION_PRECISION = 1 << 5;
        const EXCEPTION_UNUSED = 1 << 6;
        const EXCEPTION_ALL = 0b01111111;

        const PRECISION_CONTROL_24B = 0;
        const PRECISION_CONTROL_53B = 1 << 9;
        const PRECISION_CONTROL_64B = 3 << 8;

        const ROUNDING_CONTROL_NEAREST = 0;
        const ROUNDING_CONTROL_DOWN = 1 << 10;
        const ROUNDING_CONTROL_UP = 1 << 11;
        const ROUNDONG_CONTROL_TOWARD_ZERO = 3 << 10;
    }

    pub struct MXCSRFlags: u64 {
        const EXCEPTION_FLAG_INVALID_OPERATION = 1 << 0;
        const EXCEPTION_FLAG_DENORMAL_OPERAND = 1 << 1;
        const EXCEPTION_FLAG_DIVIDE_BY_ZERO = 1 << 2;
        const EXCEPTION_FLAG_OVERFLOW = 1 << 3;
        const EXCEPTION_FLAG_UNDERFLOW = 1 << 4;
        const EXCEPTION_FLAG_PRECISION = 1 << 5;

        const EXCEPTION_MASK_INVALID_OPERATION = 1 << 7;
        const EXCEPTION_MASK_DENORMAL_OPERAND = 1 << 8;
        const EXCEPTION_MASK_DIVIDE_BY_ZERO = 1 << 9;
        const EXCEPTION_MASK_OVERFLOW = 1 << 10;
        const EXCEPTION_MASK_UNDERFLOW = 1 << 11;
        const EXCEPTION_MASK_PRECISION = 1 << 12;
        const EXCEPTION_MASK_ALL = 0b1111110000000;

        const ROUNDING_TOWARDS_ZERO = 0;
        const ROUNDING_TOWARDS_NEGATIVE_INF = 1 << 13;
        const ROUNDING_TOWARDS_POSITIVE_INF = 1 << 14;

        const FLUSH_TO_ZERO = 1 << 15;
    }
}

extern "C" {
    #[link_name = "x86_64_block_task"]
    pub fn block_task();
}

pub fn get_xcr0() -> XCR0Flags {
    let upper: u64;
    let lower: u64;
    unsafe {
        asm!("xgetbv", in("rcx") 0, out("rax") lower, out("rdx") upper);
    }

    let full = upper << 32 | lower;

    XCR0Flags::from_bits(full).unwrap()
}

pub fn set_xcr0(val: XCR0Flags) {
    let upper: u64 = (val.bits & 0xffffffff00000000) >> 32;
    let lower: u64 = val.bits & 0x000000000ffffffff;
    unsafe {
        asm!("xsetbv", in("rcx") 0, in("rax") lower, in("rdx") upper);
    }
}

pub fn get_rflags() -> Rflags {
    let val: u64;
    unsafe {
        asm!("pushfq", "pop {}", out(reg) val);
    }

    Rflags::from_bits(val).unwrap()
}

pub fn get_cr0() -> CR0Flags {
    let val: u64;
    unsafe {
        asm!("mov {}, cr0", out(reg) val);
    }

    CR0Flags::from_bits(val).unwrap()
}

pub fn set_cr0(flags: CR0Flags) {
    let val = flags.bits;
    unsafe {
        asm!("mov cr0, {}", in(reg) val);
    }
}

pub fn get_cr2() -> u64 {
    let val: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) val);
    }

    val
}

pub fn set_cr3(addr: u64) {
    unsafe {
        asm!("mov cr3, {}", in(reg) addr);
    }
}

pub fn get_cr3() -> u64 {
    let val: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) val);
    }

    val
}

pub fn get_cr4() -> CR4Flags {
    let val: u64;
    unsafe {
        asm!("mov {}, cr4", out(reg) val);
    }

    CR4Flags::from_bits(val).unwrap()
}

pub fn set_cr4(flags: CR4Flags) {
    let val = flags.bits;
    unsafe {
        asm!("mov cr4, {}", in(reg) val);
    }
}

pub fn set_segment_selectors(data_selector: u64) {
    unsafe {
        asm!(
        "mov es, rax",
        "mov ds, rax",
        "mov fs, rax",
        "mov gs, rax",
        in("rax") data_selector
        );
    }
}

#[inline]
pub fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val, options(nostack, nomem));
    }
}

#[inline]
pub fn outw(port: u16, val: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") val, options(nostack, nomem));
    }
}

#[inline]
pub fn outl(port: u16, val: u32) {
    unsafe {
        asm!("out dx, eax", in("dx") port, in("eax") val, options(nostack, nomem));
    }
}

#[inline]
pub fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!("in al, dx", out("al") val, in("dx") port, options(nostack, nomem));
    }
    val
}

#[inline]
pub fn inw(port: u16) -> u16 {
    let val: u16;
    unsafe {
        asm!("in ax, dx", out("ax") val, in("dx") port, options(nostack, nomem));
    }
    val
}

#[inline]
pub fn inl(port: u16) -> u32 {
    let val: u32;
    unsafe {
        asm!("in eax, dx", out("eax") val, in("dx") port, options(nostack, nomem));
    }
    val
}

#[inline]
pub fn enable_interrupts() {
    unsafe {
        asm!("sti");
    }
}

#[inline]
pub fn disable_interrupts() {
    unsafe {
        asm!("cli");
    }
}

#[inline]
pub fn interrupts_enabled() -> bool {
    let rflags = get_rflags();
    rflags.contains(Rflags::INTERRUPT)
}

pub fn get_current_pml4_phys() -> PhysAddr {
    PhysAddr::new(get_cr3())
}

pub fn get_current_pml4() -> PML4 {
    PML4::from_phys(get_current_pml4_phys())
}

#[inline]
pub fn flush_tlb_page(virt: u64) {
    unsafe {
        asm!("invlpg [{}]", in(reg) virt);
    }
}

pub fn fldcw(flags: X87Flags) {
    let val = flags.bits;
    unsafe {
        asm!("fldcw [{}]", in(reg) &val);
    }
}

pub fn load_mxcsr(flags: MXCSRFlags) {
    let val = flags.bits;
    unsafe {
        asm!("ldmxcsr [{}]", in(reg) &val);
    }
}

pub fn init() {
    let mut cr0 = get_cr0();
    cr0.remove(CR0Flags::EM);
    cr0.insert(CR0Flags::MP);
    set_cr0(cr0);

    let mut cr4 = get_cr4();
    cr4.insert(CR4Flags::OSFXSR);
    cr4.insert(CR4Flags::OSXMMEXCPT);
    set_cr4(cr4);

    fldcw(
        X87Flags::EXCEPTION_ALL
            | X87Flags::PRECISION_CONTROL_64B
            | X87Flags::ROUNDING_CONTROL_NEAREST,
    );

    load_mxcsr(MXCSRFlags::EXCEPTION_MASK_ALL | MXCSRFlags::ROUNDING_TOWARDS_ZERO);

    //let mut xcr0 = get_xcr0();
    //xcr0.insert(XCR0Flags::SSE);
    //xcr0.insert(XCR0Flags::X87);
    //set_xcr0(xcr0);
}
