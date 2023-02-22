use core::str::from_utf8;

use alloc::{slice, sync::Arc};
use spin::Mutex;

use crate::{framebuffer, scheduler::proc::Process};

pub fn sys_write(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let p = proc.lock();

    println!("{:?}", args);
    let fd = args[0];
    let len = args[2];
    let buff = unsafe { slice::from_raw_parts(args[1] as *mut u8, len as usize) };
    println!("{:#x}", args[1]);

    println!("buff: `{:?}`", buff);

    let str = from_utf8(buff).unwrap();
    framebuffer::draw_text(str, 0, 0);

    0
}