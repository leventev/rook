use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    arch::x86_64::{
        self, disable_interrupts, enable_interrupts,
        idt::{self, IDTTypeAttr},
        registers::InterruptRegisters,
    },
    scheduler::{
        proc::{get_process, Process},
        thread::ThreadInner,
        SCHEDULER,
    },
};

type SyscallCallback = fn(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64;

pub struct Syscall {
    name: &'static str,
    callback: SyscallCallback,
}

impl Syscall {
    const fn new(name: &'static str, callback: SyscallCallback) -> Syscall {
        Syscall { name, callback }
    }
}

static SYSCALL_TABLE: &[Syscall] = &[
    Syscall::new("write", x86_64::syscall::io::sys_write),
    Syscall::new("read", x86_64::syscall::io::sys_read),
    Syscall::new("openat", x86_64::syscall::io::sys_openat),
    Syscall::new("close", x86_64::syscall::io::sys_close),
    Syscall::new("fstatat", x86_64::syscall::io::sys_fstatat),
    Syscall::new("mmap", x86_64::syscall::mm::sys_mmap),
    Syscall::new("getpid", x86_64::syscall::proc::sys_getpid),
    Syscall::new("getppid", x86_64::syscall::proc::sys_getppid),
    Syscall::new("getuid", x86_64::syscall::proc::sys_getuid),
    Syscall::new("geteuid", x86_64::syscall::proc::sys_geteuid),
    Syscall::new("getgid", x86_64::syscall::proc::sys_getgid),
    Syscall::new("getegid", x86_64::syscall::proc::sys_getegid),
    Syscall::new("getcwd", x86_64::syscall::proc::sys_getcwd),
    Syscall::new("fcntl", x86_64::syscall::io::sys_fcntl),
    Syscall::new("ioctl", x86_64::syscall::io::sys_ioctl),
    Syscall::new("getpgid", x86_64::syscall::proc::sys_getpgid),
    Syscall::new("setpgid", x86_64::syscall::proc::sys_setpgid),
    Syscall::new("clone", x86_64::syscall::proc::sys_clone),
];

#[no_mangle]
fn handle_syscall(interrupt_regs: &mut InterruptRegisters) {
    let syscall_no: u64;
    let args: [u64; 6];

    let thread_lock = SCHEDULER.get_current_thread().expect("No threads running");
    let pid: usize;
    let process = {
        let mut current_thread = thread_lock.lock();

        if let ThreadInner::User(data) = &mut current_thread.inner {
            syscall_no = interrupt_regs.general.rax;
            args = [
                interrupt_regs.general.rdi,
                interrupt_regs.general.rsi,
                interrupt_regs.general.rdx,
                interrupt_regs.general.r10,
                interrupt_regs.general.r8,
                interrupt_regs.general.r9,
            ];

            data.user_regs.general = interrupt_regs.general;
            data.user_regs.rip = interrupt_regs.iret.rip;
            data.user_regs.rsp = interrupt_regs.iret.rsp;
            data.user_regs.selectors.ss = interrupt_regs.iret.ss;
            data.user_regs.selectors.cs = interrupt_regs.iret.cs;

            data.in_kernelspace = true;
            pid = data.pid;
            get_process(data.pid).unwrap()
        } else {
            unreachable!()
        }
    };

    let syscall_table_idx = syscall_no as usize;
    assert!(syscall_table_idx < SYSCALL_TABLE.len());

    enable_interrupts();

    let syscall = &SYSCALL_TABLE[syscall_table_idx];
    println!("handle syscall PID: {} {} {:?}", pid, syscall.name, args);

    let res = (syscall.callback)(process, args);
    println!("syscall return {:#x}", res);

    disable_interrupts();

    {
        let mut current_thread = thread_lock.lock();

        if let ThreadInner::User(data) = &mut current_thread.inner {
            data.in_kernelspace = false;
        } else {
            unreachable!()
        }
    }

    interrupt_regs.general.rax = res;
}

extern "C" {
    fn __handle_syscall();
}

pub fn init() {
    let idt_type = IDTTypeAttr::INTERRUPT_GATE | IDTTypeAttr::RING3 | IDTTypeAttr::PRESENT;
    let callback = __handle_syscall as u64;
    idt::install_interrupt_handler(0x80, callback, idt_type, 3);
}
