#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(default_free_fn)]
#![allow(dead_code)]
#![feature(alloc_error_handler)]
#![feature(generic_arg_infer)]

extern crate alloc;

#[macro_use]
mod io;
mod arch;
mod drivers;
mod mm;
mod time;

use limine::{
    LimineBootInfoRequest, LimineBootTimeRequest, LimineHhdmRequest, LimineMemmapRequest,
};

use crate::arch::x86_64::{disable_interrupts, enable_interrupts, idt, pic};

static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static MMAP_INFO: LimineMemmapRequest = LimineMemmapRequest::new(0);
static HHDM_INFO: LimineHhdmRequest = LimineHhdmRequest::new(0);
static BOOT_TIME_INFO: LimineBootTimeRequest = LimineBootTimeRequest::new(0);

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("hello, world Rook!");

    if let Some(bootinfo) = BOOTLOADER_INFO.get_response().get() {
        println!(
            "booted by {} v{}",
            bootinfo.name.to_str().unwrap().to_str().unwrap(),
            bootinfo.version.to_str().unwrap().to_str().unwrap(),
        );
    }

    mm::phys::init(
        MMAP_INFO
            .get_response()
            .get()
            .expect("Memory map request failed"),
    );

    let hhdm = HHDM_INFO
        .get_response()
        .get()
        .expect("HHDM request failed")
        .offset;

    mm::virt::init(hhdm);
    mm::virt::dump_pml4();

    idt::init();
    pic::init();

    let boot_time = BOOT_TIME_INFO
        .get_response()
        .get()
        .expect("BOOT TIME request failed")
        .boot_time;

    time::init(boot_time as u64);
    drivers::pit::init();

    mm::kalloc::init();

    hcf();
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    hcf();
}

/// Die, spectacularly.
pub fn hcf() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
