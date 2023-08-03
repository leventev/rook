use core::ffi::{c_char, CStr};

use alloc::{slice, string::String, sync::Arc, vec::Vec};
use bitflags::bitflags;
use spin::Mutex;

use crate::{
    arch::x86_64::disable_interrupts,
    posix::errno,
    scheduler::{
        self,
        proc::{get_process, Process},
        thread::{ThreadID, ThreadInner},
        SCHEDULER,
    },
};

bitflags! {
    pub struct CloneFlags: u64 {
        const CLONE_FILES = 1 << 0;
        const CLONE_VM = 1 << 1;
        const CLONE_VFORK = 1 << 2;
    }
}

pub struct CloneArgs {
    pub flags: u64,
    pub pidfd: u64,

    pub child_tid: u64,

    pub parent_tid: u64,

    pub exit_signal: u64,

    pub stack: u64,
    pub stack_size: u64,
    pub tls: u64,
    pub set_tid: u64,

    pub set_tid_size: u64,

    pub cgroup: u64,
}

#[derive(Debug, Clone, Copy)]
enum SyscallProcError {
    InvalidPID,
    NotFoundPID,
}

impl SyscallProcError {
    fn as_errno(&self) -> u64 {
        let val = match self {
            Self::InvalidPID => errno::EINVAL,
            Self::NotFoundPID => errno::ESRCH,
        };

        (-val) as u64
    }
}

pub fn sys_getpid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().pid as u64
}

pub fn sys_getppid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().ppid as u64
}

pub fn sys_getuid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().uid as u64
}

pub fn sys_geteuid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().euid as u64
}

pub fn sys_getgid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().gid as u64
}

pub fn sys_getegid(proc: Arc<Mutex<Process>>, _args: [u64; 6]) -> u64 {
    proc.lock().egid as u64
}

pub fn sys_getcwd(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let len = args[1] as usize;
    let buff = unsafe { slice::from_raw_parts_mut(args[0] as *mut u8, len) };

    match getcwd(proc, buff) {
        Ok(_) => args[0],
        Err(_) => 0,
    }
}

fn getcwd(proc: Arc<Mutex<Process>>, buff: &mut [u8]) -> Result<(), ()> {
    let p = proc.lock();
    let vnode = &p.cwd.lock().vnode;
    let vnode_path = vnode.path();

    if vnode_path.len() > buff.len() {
        return Err(());
    }

    let buff = &mut buff[..vnode_path.len() + 1];
    buff[..vnode_path.len()].copy_from_slice(vnode_path.as_bytes());
    buff[buff.len() - 1] = b'\0';

    Ok(())
}

pub fn sys_getpgid(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let pid = args[0] as isize;

    match getpgid(proc, pid) {
        Ok(gpid) => gpid as u64,
        Err(err) => err.as_errno(),
    }
}

fn getpgid(proc: Arc<Mutex<Process>>, pid: isize) -> Result<usize, SyscallProcError> {
    if pid < 0 {
        return Err(SyscallProcError::InvalidPID);
    }

    if pid == 0 {
        return Ok(proc.lock().pgid);
    }

    match scheduler::proc::get_process(pid as usize) {
        Some(proc) => Ok(proc.lock().pgid),
        None => Err(SyscallProcError::NotFoundPID),
    }
}

pub fn sys_setpgid(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let pid = args[0] as usize;
    let pgid = args[1] as usize;

    match setpgid(proc, pid, pgid) {
        Ok(_) => 0,
        Err(err) => err.as_errno(),
    }
}

fn setpgid(proc: Arc<Mutex<Process>>, pid: usize, pgid: usize) -> Result<(), SyscallProcError> {
    // TODO: session leader checks, etc...
    let p = if pid == 0 {
        proc
    } else {
        match get_process(pid) {
            Some(p) => p,
            None => return Err(SyscallProcError::InvalidPID),
        }
    };

    let mut p = p.lock();
    let new_pgid = if pgid == 0 { p.pid } else { pgid };

    p.pgid = new_pgid;

    Ok(())
}

pub fn sys_clone(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let clone_args = args[0] as *const CloneArgs;
    let size = args[1] as usize;

    match clone(proc, clone_args, size) {
        Ok(pid) => pid as u64,
        Err(err) => err.as_errno(),
    }
}

fn clone(
    proc: Arc<Mutex<Process>>,
    clone_args: *const CloneArgs,
    _size: usize,
) -> Result<usize, SyscallProcError> {
    // TODO: check if sizeof(clone_args) == size???
    // TODO: validate clone_args

    let child_tid: ThreadID;
    let child_pid: usize;
    let block_wait_for_child: bool;

    {
        let clone_args = unsafe { clone_args.as_ref() }.unwrap();
        let p = proc.lock();

        let child = p.clone_proc(clone_args);
        let child = child.lock();
        child_pid = child.pid;

        {
            let thread = child.main_thread.upgrade().unwrap();
            let mut thread = thread.lock();

            child_tid = thread.id;

            if let ThreadInner::User(data) = &mut thread.inner {
                data.user_regs.general.rax = 0;
                data.in_kernelspace = false;
            }
        }

        let clone_flags = CloneFlags::from_bits(clone_args.flags).unwrap();
        block_wait_for_child = clone_flags.contains(CloneFlags::CLONE_VFORK);
    }

    // TODO: disable interrupts?, maybe scheduler interrupt mutex already does that for us
    SCHEDULER.run_thread(child_tid);

    if block_wait_for_child {
        SCHEDULER.block_current_thread();
    }

    Ok(child_pid)
}

pub fn sys_execve(proc: Arc<Mutex<Process>>, args: [u64; 6]) -> u64 {
    let path = args[0] as *const c_char;
    let argv = args[1] as *const *const c_char;
    let envp = args[2] as *const *const c_char;

    match execve(proc, path, argv, envp) {
        Ok(_) => 0,
        Err(err) => err.as_errno(),
    }
}

unsafe fn parse_c_char_array(arr: *const *const c_char) -> Vec<String> {
    let mut vec = Vec::new();

    // TODO: error handling
    // TODO: work with bytes instead of strings

    let mut ptr = arr;
    let mut c_str = *ptr;
    while !c_str.is_null() {
        let str = CStr::from_ptr(c_str).to_str().unwrap();
        let str = String::from(str);
        vec.push(str);

        ptr = ptr.add(1);
        c_str = *ptr;
    }

    vec
}

fn execve(
    proc: Arc<Mutex<Process>>,
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> Result<(), SyscallProcError> {
    // TODO: errors
    disable_interrupts();
    let mut p = proc.lock();

    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap();

    // TODO: optimize this
    let argv_vec = unsafe { parse_c_char_array(argv) };
    let argv_vec: Vec<&str> = argv_vec.iter().map(|s| s.as_str()).collect();

    let envp_vec = unsafe { parse_c_char_array(envp) };
    let envp_vec: Vec<&str> = envp_vec.iter().map(|s| s.as_str()).collect();

    debug!("{} {:?} {:?}", path, argv_vec, envp_vec);

    p.execve(path, &argv_vec, &envp_vec)
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
