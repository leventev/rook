use core::arch::asm;

const MAX_FRAMES: usize = 64;

pub fn walk() {
    let mut rbp: usize;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }

    log!("stack trace:");

    for _ in 0..MAX_FRAMES {
        if rbp == 0 {
            return;
        }
        let func = unsafe { *(rbp as *const usize).offset(1) };
        log!("  {:#x}", func);
        rbp = unsafe { *(rbp as *const usize) };
    }
}
