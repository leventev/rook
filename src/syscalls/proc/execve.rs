use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Mutex;

use crate::{
    arch::x86_64::disable_interrupts,
    posix::errno::Errno,
    scheduler::{proc::Process, thread::ThreadInner},
};

pub fn execve(
    proc: Arc<Mutex<Process>>,
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<(), Errno> {
    // TODO: errors
    disable_interrupts();
    let mut p = proc.lock();

    let argv: Vec<&str> = argv.iter().map(String::as_ref).collect();
    let envp: Vec<&str> = envp.iter().map(String::as_ref).collect();

    p.execve(path, &argv, &envp)
        .expect("Failed to load process");

    let main_thread_lock = p.main_thread.upgrade().unwrap();
    let mut main_thread = main_thread_lock.lock();

    // load_from_file already sets rip, rsp and (argc)rdi, (argv)rsi, (envp)rdx
    if let ThreadInner::User(data) = &mut main_thread.inner {
        data.user_regs.general.rax = 0;
        data.user_regs.general.rbx = 0;
        data.user_regs.general.rcx = 0;
        data.user_regs.general.r8 = 0;
        data.user_regs.general.r9 = 0;
        data.user_regs.general.r10 = 0;
        data.user_regs.general.r11 = 0;
        data.user_regs.general.r12 = 0;
        data.user_regs.general.r13 = 0;
        data.user_regs.general.r14 = 0;
        data.user_regs.general.r15 = 0;
        data.user_regs.general.rbp = 0;
    }

    Ok(())
}
