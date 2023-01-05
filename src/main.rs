#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(default_free_fn)]

#[macro_use]
mod io;
pub mod mm;

use limine::{LimineBootInfoRequest, LimineMemmapRequest};

static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static MMAP_INFO: LimineMemmapRequest = LimineMemmapRequest::new(0);

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("hello, world Ruke!");

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
