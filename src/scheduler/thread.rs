use alloc::{boxed::Box, fmt, sync::Arc, sync::Weak, vec::Vec};
use spin::Mutex;

use crate::{
    arch::x86_64::{
        gdt::{segment_selector, GDT_KERNEL_CODE, GDT_KERNEL_DATA, GDT_USER_CODE, GDT_USER_DATA},
        get_current_pml4, interrupts_enabled,
        paging::PageFlags,
        Rflags,
    },
    mm::{
        phys,
        virt::{KERNEL_THREAD_STACKS_START, VIRTUAL_MEMORY_MANAGER},
        PhysAddr, VirtAddr,
    },
    scheduler::remove_current_thread_wrapper,
};

#[repr(transparent)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ThreadID(pub usize);

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct RegisterState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    pub rbp: u64,

    pub es: u64,
    pub ds: u64,
    pub fs: u64,
    pub gs: u64,

    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl RegisterState {
    pub const fn zero() -> RegisterState {
        RegisterState {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbp: 0,
            es: 0,
            ds: 0,
            fs: 0,
            gs: 0,
            rip: 0,
            cs: 0,
            rflags: 0,
            rsp: 0,
            ss: 0,
        }
    }

    fn kernel_new() -> RegisterState {
        RegisterState {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbp: 0,
            es: segment_selector(GDT_KERNEL_DATA, 0),
            ds: segment_selector(GDT_KERNEL_DATA, 0),
            fs: segment_selector(GDT_KERNEL_DATA, 0),
            gs: segment_selector(GDT_KERNEL_DATA, 0),
            rip: 0,
            cs: segment_selector(GDT_KERNEL_CODE, 0),
            rflags: (Rflags::INTERRUPT | Rflags::RESERVED_BIT_1).bits(),
            // FIXME: this may not be the best option ^^^^
            rsp: 0,
            ss: segment_selector(GDT_KERNEL_DATA, 0),
        }
    }

    fn user_new() -> RegisterState {
        RegisterState {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbp: 0,
            es: segment_selector(GDT_USER_DATA, 3),
            ds: segment_selector(GDT_USER_DATA, 3),
            fs: segment_selector(GDT_USER_DATA, 3),
            gs: segment_selector(GDT_USER_DATA, 3),
            rip: 0,
            cs: segment_selector(GDT_USER_CODE, 3),
            rflags: (Rflags::INTERRUPT | Rflags::RESERVED_BIT_1).bits(),
            // FIXME: this may not be the best option ^^^^
            rsp: 0,
            ss: segment_selector(GDT_USER_DATA, 3),
        }
    }
}

impl fmt::Display for RegisterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rax: u64 = self.rax;
        let rbx: u64 = self.rbx;
        let rcx: u64 = self.rcx;
        let rdx: u64 = self.rdx;
        let rsi: u64 = self.rsi;
        let rdi: u64 = self.rdi;
        let r8: u64 = self.r8;
        let r9: u64 = self.r9;
        let r10: u64 = self.r10;
        let r11: u64 = self.r11;
        let r12: u64 = self.r12;
        let r13: u64 = self.r13;
        let r14: u64 = self.r14;
        let r15: u64 = self.r15;
        let rbp: u64 = self.rbp;
        let es: u64 = self.es;
        let ds: u64 = self.ds;
        let fs: u64 = self.fs;
        let gs: u64 = self.gs;
        let rip: u64 = self.rip;
        let cs: u64 = self.cs;
        let rflags: u64 = self.rflags;
        let rsp: u64 = self.rsp;
        let ss: u64 = self.ss;

        writeln!(
            f,
            "RAX={rax:0>16x} RBX={rbx:0>16x} RCX={rcx:0>16x} RDX={rdx:0>16x}"
        )?;
        writeln!(
            f,
            "RSI={rsi:0>16x} RDI={rdi:0>16x}  R8={r8:0>16x}  R9={r9:0>16x}"
        )?;
        writeln!(
            f,
            "R10={r10:0>16x} R11={r11:0>16x} R12={r12:0>16x} R13={r13:0>16x}"
        )?;
        writeln!(f, "R14={r14:0>16x} R15={r15:0>16x}")?;
        writeln!(f, "RIP={rip:0>16x} RBP={rbp:0>16x} RSP={rsp:0>16x}")?;
        writeln!(
            f,
            "RFLAGS={rflags:0>16x}({:?})",
            Rflags::from_bits(rflags).unwrap()
        )?;
        write!(
            f,
            "ES={es:0>4x} DS={ds:0>4x} FS={fs:0>4x} GS={gs:0>4x} SS={ss:0>4x} CS={cs:0>4x}"
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadState {
    None,
    Running,
    Busy,
}

#[derive(Debug, Clone)]
pub struct KernelThreadData {
    pub regs: Box<RegisterState>,
}

#[derive(Debug, Clone)]
pub struct UserThreadData {
    pub pid: usize,
    pub kernel_regs: Box<RegisterState>,
    pub user_regs: Box<RegisterState>,
    pub in_kernelspace: bool,
}

#[derive(Debug, Clone)]
pub enum ThreadInner {
    Kernel(KernelThreadData),
    User(UserThreadData),
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub id: ThreadID,
    pub state: ThreadState,
    pub stack_bottom: u64,
    pub inner: ThreadInner,
}

pub struct SchedulerThreadData {
    threads: Vec<Option<Arc<Mutex<Thread>>>>,
    // TODO: try to fill the queue without exposing running_threads as public
    pub running_threads: Vec<ThreadID>,
    busy_threads: Vec<ThreadID>,
    thread_count: usize,
}

const KERNEL_STACK_SIZE_PER_THREAD: u64 = 2 * 4096; // 8KiB
const MAX_THREADS: usize = 64;

impl SchedulerThreadData {
    fn get_kernel_stack(tid: ThreadID) -> u64 {
        // FIXME: increase limit
        assert!(tid.0 < MAX_THREADS);
        KERNEL_THREAD_STACKS_START.get() + tid.0 as u64 * KERNEL_STACK_SIZE_PER_THREAD
    }

    pub fn init(&mut self) {
        assert!(!interrupts_enabled());
        let vmm = VIRTUAL_MEMORY_MANAGER.lock();

        const ALLOC_AT_ONCE: usize = 128;
        let kernel_stack_space_size = MAX_THREADS as u64 * KERNEL_STACK_SIZE_PER_THREAD;
        let in_pages = kernel_stack_space_size / 4096;

        assert!(in_pages % ALLOC_AT_ONCE as u64 == 0);
        let allocs = in_pages as usize / ALLOC_AT_ONCE;

        for i in 0..allocs {
            let phys_start = phys::alloc_multiple(ALLOC_AT_ONCE);
            for j in 0..ALLOC_AT_ONCE {
                let phys = phys_start + PhysAddr::new(j as u64 * 4096);
                let virt = KERNEL_THREAD_STACKS_START
                    + VirtAddr::new((i * ALLOC_AT_ONCE + j) as u64 * 4096);
                vmm.map_4kib(
                    get_current_pml4(),
                    virt,
                    phys,
                    PageFlags::PRESENT | PageFlags::READ_WRITE,
                );
            }
        }

        self.threads.resize(16, None);
    }

    fn alloc_tid(&mut self) -> ThreadID {
        let tid = if self.thread_count == self.threads.capacity() {
            let old_size = self.threads.capacity() * 2;
            self.threads.resize(old_size * 2, None);
            old_size
        } else {
            self.threads.iter().position(Option::is_none).unwrap()
        };

        self.thread_count += 1;

        ThreadID(tid)
    }

    pub fn new_kernel_thread(&mut self) -> Thread {
        let tid = self.alloc_tid();
        Thread {
            id: tid,
            state: ThreadState::None,
            inner: ThreadInner::Kernel(KernelThreadData {
                regs: Box::new(RegisterState::kernel_new()),
            }),
            stack_bottom: Self::get_kernel_stack(tid) + KERNEL_STACK_SIZE_PER_THREAD,
        }
    }

    /// spawns a kernel thread and returns the thread id
    pub fn create_kernel_thread(&mut self, func: fn()) -> Weak<Mutex<Thread>> {
        let tid: ThreadID;
        let thread = Arc::new(Mutex::new({
            let mut thread = self.new_kernel_thread();
            tid = thread.id;

            if let ThreadInner::Kernel(data) = &mut thread.inner {
                // push the address of remove_running_thread on the stack so the thread
                // will return to that address and get killed
                data.regs.rsp = thread.stack_bottom;
                data.regs.rsp -= core::mem::size_of::<u64>() as u64;
                unsafe {
                    *(data.regs.rsp as *mut u64) = remove_current_thread_wrapper as usize as u64;
                }

                data.regs.rip = func as usize as u64;
                thread
            } else {
                unreachable!()
            }
        }));

        let weak = Arc::downgrade(&thread);
        self.threads[tid.0] = Some(thread);

        self.change_thread_state(tid, ThreadState::Running);

        println!("spawn kernel thread: {:#x}", tid.0);
        weak
    }

    pub fn new_user_thread(&mut self, pid: usize) -> Thread {
        let tid = self.alloc_tid();
        Thread {
            id: tid,
            state: ThreadState::None,
            stack_bottom: Self::get_kernel_stack(tid),
            inner: ThreadInner::User(UserThreadData {
                pid,
                kernel_regs: Box::new(RegisterState::user_new()),
                user_regs: Box::new(RegisterState::user_new()),
                in_kernelspace: false,
            }),
        }
    }

    pub fn create_user_thread(&mut self, pid: usize) -> Weak<Mutex<Thread>> {
        let tid: ThreadID;
        let thread = Arc::new(Mutex::new({
            let thread = self.new_user_thread(pid);
            tid = thread.id;
            thread
        }));

        let weak = Arc::downgrade(&thread);
        self.threads[tid.0] = Some(thread);

        println!("spawn user thread: {:#x}", tid.0);
        weak
    }

    pub fn copy_user_thread(&mut self, pid: usize, tid: ThreadID) -> Weak<Mutex<Thread>> {
        let new_tid = self.alloc_tid();

        let new_thread = Arc::new(Mutex::new({
            let old_thread = self.threads[tid.0].as_ref().expect("Invalid TID");
            let old_thread = old_thread.lock();

            let mut thread = old_thread.clone();
            thread.id = new_tid;
            // TODO: copy pid
            thread
        }));

        let weak = Arc::downgrade(&new_thread);
        self.threads[new_tid.0] = Some(new_thread);

        println!("copy_user_thread: {:#x}", tid.0);
        weak
    }

    fn add_to_running_threads(&mut self, tid: ThreadID) {
        self.running_threads.push(tid);
    }

    fn add_to_busy_threads(&mut self, tid: ThreadID) {
        self.busy_threads.push(tid);
    }

    fn remove_from_running_threads(&mut self, tid: ThreadID) {
        let idx = self
            .running_threads
            .iter()
            .position(|thread_id| *thread_id == tid)
            .expect("Invalid TID");
        self.running_threads.remove(idx);
    }

    fn remove_from_busy_threads(&mut self, tid: ThreadID) {
        let idx = self
            .busy_threads
            .iter()
            .position(|thread_id| *thread_id == tid)
            .expect("Invalid TID");
        self.busy_threads.remove(idx);
    }

    pub fn get_thread(&self, tid: ThreadID) -> Option<Arc<Mutex<Thread>>> {
        self.threads[tid.0].as_ref().map(|t| t.clone())
    }

    pub fn remove_thread(&mut self, tid: ThreadID) {
        let thread = self.get_thread(tid).expect("Invalid TID");
        let thread = thread.lock();

        match thread.state {
            ThreadState::Busy => self.remove_from_busy_threads(tid),
            ThreadState::Running => self.remove_from_running_threads(tid),
            _ => unreachable!(),
        };

        self.threads[tid.0] = None;
        self.thread_count -= 1;
    }

    pub fn change_thread_state(&mut self, tid: ThreadID, new_state: ThreadState) {
        let thread = self.get_thread(tid).expect("Invalid TID");
        let mut thread = thread.lock();
        
        assert!(
            thread.state != new_state,
            "Trying to change thread state to current state"
        );

        let prev_state = thread.state;
        
        match new_state {
            ThreadState::Busy => {
                if prev_state == ThreadState::Running {
                    self.remove_from_running_threads(tid);
                }
                self.add_to_busy_threads(tid);
            }
            ThreadState::Running => {
                if prev_state == ThreadState::Busy {
                    self.remove_from_busy_threads(tid);
                }
                self.add_to_running_threads(tid);
            }
            _ => unreachable!(),
        }
        thread.state = new_state;
    }

    pub const fn new() -> Self {
        SchedulerThreadData {
            threads: Vec::new(),
            running_threads: Vec::new(),
            busy_threads: Vec::new(),
            thread_count: 0,
        }
    }
}
