use crate::arch::x86_64::{
    self,
    idt::{self, IDTTypeAttr},
};

pub struct Syscall {
    name: &'static str,
    callback: fn(args: [u64; 6]) -> u64,
}

impl Syscall {
    const fn new(name: &'static str, callback: fn(args: [u64; 6]) -> u64) -> Syscall {
        Syscall { name, callback }
    }
}

static SYSCALL_TABLE: [Syscall; 1] = [Syscall::new("write", x86_64::syscall::io::sys_write)];

#[no_mangle]
fn handle_syscall(
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
    syscall_no: u64,
) -> u64 {
    let syscall_table_idx = syscall_no as usize;
    assert!(syscall_table_idx < SYSCALL_TABLE.len());

    let syscall = &SYSCALL_TABLE[syscall_table_idx];
    let args = [arg1, arg2, arg3, arg4, arg5, arg6];
    println!("handle syscall {}", syscall.name);

    let res = (syscall.callback)(args);
    println!("syscall return {:#x}", res);
    res
}

extern "C" {
    fn __handle_syscall();
}

pub fn init() {
    let idt_type = IDTTypeAttr::INTERRUPT_GATE | IDTTypeAttr::RING3 | IDTTypeAttr::PRESENT;
    let callback = __handle_syscall as u64;
    idt::install_interrupt_handler(0x80, callback, idt_type, 3);
}
