#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(default_free_fn)]
#![allow(dead_code)]
#![feature(alloc_error_handler)]
#![feature(generic_arg_infer)]
#![feature(linked_list_cursors)]
#![feature(new_uninit)]
#![feature(const_ptr_as_ref)]
#![feature(const_option)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod io;
mod arch;
mod blk;
mod dma;
mod drivers;
mod framebuffer;
mod fs;
mod mm;
mod pci;
mod scheduler;
mod time;
mod utils;

use alloc::{slice, string::String};
use limine::{
    LimineBootTimeRequest, LimineFramebufferRequest, LimineHhdmRequest, LimineMemmapRequest,
};

use crate::{
    arch::x86_64::{idt, pic, stacktrace},
    mm::{virt::HDDM_VIRT_START, VirtAddr},
    scheduler::proc,
};

static MMAP_INFO: LimineMemmapRequest = LimineMemmapRequest::new(0);
static HHDM_INFO: LimineHhdmRequest = LimineHhdmRequest::new(0);
static BOOT_TIME_INFO: LimineBootTimeRequest = LimineBootTimeRequest::new(0);
static FRAMEBUFFER_INFO: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

#[no_mangle]
fn vmm_setup() {
    let hhdm = HHDM_INFO
        .get_response()
        .get()
        .expect("HHDM request failed")
        .offset;

    let mmap = MMAP_INFO
        .get_response()
        .get()
        .expect("Memory map request failed");

    let framebuffer = FRAMEBUFFER_INFO
        .get_response()
        .get()
        .expect("Framebuffer request failed");

    println!("{} framebuffers available", framebuffer.framebuffer_count);
    assert!(framebuffer.framebuffer_count > 0);
    let framebuffers = unsafe {
        slice::from_raw_parts_mut(
            framebuffer.framebuffers.as_ptr(),
            framebuffer.framebuffer_count as usize,
        )
    };

    let fb = &mut framebuffers[0];

    // FIXME
    // FIXME
    // FIXME
    // FIXME
    let buff_phys = (fb.address.as_ptr().unwrap() as u64) - hhdm;

    framebuffer::init(
        VirtAddr::new(HDDM_VIRT_START.get() + buff_phys),
        fb.width as usize,
        fb.height as usize,
        fb.pitch as usize,
        fb.bpp as usize,
    );

    mm::virt::init(hhdm);
    mm::phys::init(mmap);

    // this function sets the rsp
    mm::virt::map_physical_address_space();
}

#[no_mangle]
fn kernel_init() -> ! {
    let boot_time = BOOT_TIME_INFO
        .get_response()
        .get()
        .expect("BOOT TIME request failed")
        .boot_time;

    // only unmap it after every we executed every request
    mm::virt::unmap_limine_pages();

    idt::init();
    pic::init();

    time::init(boot_time as u64);

    mm::kalloc::init();

    scheduler::init();
    scheduler::spawn_kernel_thread(main_init_thread);
    scheduler::start();
}

fn main_init_thread() {
    drivers::init();

    drivers::preload_driver("serial");
    drivers::preload_driver("pit");

    pci::init();

    drivers::load_drivers();

    fs::init();

    let part = blk::get_partition(1, 0, 0).unwrap();
    fs::mount(String::from("/"), part, "FAT").unwrap();

    // we have to initialize the font after kalloc has been initialized
    framebuffer::init_font();
    framebuffer::draw_text("test", 0, 0);

    println!("main init thread");
    proc::load_base_process("/bin/test");
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    stacktrace::walk();
    println!("{}", info);
    hcf();
}

/// Die, spectacularly.
pub fn hcf() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
