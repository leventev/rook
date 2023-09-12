#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![allow(dead_code)]
#![feature(alloc_error_handler)]
#![feature(generic_arg_infer)]
#![feature(linked_list_cursors)]
#![feature(new_uninit)]
#![feature(const_ptr_as_ref)]
#![feature(const_option)]
#![feature(format_args_nl)]
#![feature(int_roundings)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod logger;
mod arch;
mod blk;
mod console;
mod dma;
mod drivers;
mod framebuffer;
mod fs;
mod mm;
mod pci;
mod posix;
mod scheduler;
mod sync;
mod syscall;
mod time;
mod utils;

use alloc::slice;
use arch::x86_64::{self, gdt};
use fs::VFS;
use limine::{BootTimeRequest, FramebufferRequest, HhdmRequest, MemmapRequest};
use scheduler::SCHEDULER;

use crate::{
    arch::x86_64::{disable_interrupts, get_current_pml4, idt, pic, stacktrace},
    fs::devfs,
    mm::{virt::HDDM_VIRT_START, VirtAddr},
    scheduler::proc,
};

static MMAP_INFO: MemmapRequest = MemmapRequest::new(0);
static HHDM_INFO: HhdmRequest = HhdmRequest::new(0);
static BOOT_TIME_INFO: BootTimeRequest = BootTimeRequest::new(0);
static FRAMEBUFFER_INFO: FramebufferRequest = FramebufferRequest::new(0);

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

    log!("{} framebuffers available", framebuffer.framebuffer_count);
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

    let pml4 = get_current_pml4();

    pml4.map_hhdm(VirtAddr::new(hhdm));
    mm::phys::init(mmap);

    pml4.map_physical_address_space();
}

#[no_mangle]
fn kernel_init() -> ! {
    let boot_time = BOOT_TIME_INFO
        .get_response()
        .get()
        .expect("BOOT TIME request failed")
        .boot_time;

    // only unmap it after every we executed every request
    let pml4 = get_current_pml4();
    pml4.unmap_limine_pages();

    x86_64::init();

    gdt::init();

    idt::init();
    pic::init();

    time::init(boot_time as u64);

    mm::kalloc::init(&pml4);

    mm::phys::init_page_descriptors();

    SCHEDULER.init(&pml4);
    SCHEDULER.create_kernel_thread(main_init_thread);
    SCHEDULER.start();
}

fn main_init_thread() {
    drivers::init();

    drivers::preload_driver("serial");
    drivers::preload_driver("pit");

    pci::init();

    drivers::load_drivers();

    {
        let mut vfs = VFS.write();
        let part = blk::get_partition(1, 0, 0).unwrap();
        vfs.mount("/", part, "fat32").unwrap();
    }

    devfs::init();
    console::init();

    // we have to initialize the font after kalloc has been initialized
    framebuffer::init_font();

    syscall::init();

    proc::load_base_process("/bin/bash");
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    disable_interrupts();

    stacktrace::walk();
    error!("{}", info);
    hcf();
}

/// Die, spectacularly.
pub fn hcf() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
