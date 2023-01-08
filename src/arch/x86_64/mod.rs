pub mod exception;
pub mod idt;
pub mod paging;
pub mod pic;

use core::arch::asm;

extern "C" {
    #[link_name = "x86_64_get_cr3"]
    pub fn get_cr3() -> u64;

    #[link_name = "x86_64_set_cr3"]
    pub fn set_cr3(cr3: u64);

    #[link_name = "x86_64_get_rflags"]
    pub fn get_rflags() -> u64;
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
    unsafe { asm!("sti"); }
}

#[inline]
pub fn disable_interrupts() {
    unsafe { asm!("cli"); }
}
