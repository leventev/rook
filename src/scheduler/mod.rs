pub mod proc;

use crate::{
    arch::x86_64::{
        self,
        gdt::{segment_selector, GDT_KERNEL_CODE, GDT_KERNEL_DATA, GDT_USER_CODE, GDT_USER_DATA},
        paging::PageFlags,
    },
    mm::{
        phys,
        virt::{self, KERNEL_THREAD_STACKS_START},
        PhysAddr, VirtAddr,
    },
};
use core::arch::asm;

use core::fmt;

use alloc::{
    boxed::Box,
    collections::VecDeque,
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::Mutex;

// kernel thread IDs in the kernel are different from the PIDs of processes/threads
// a thread may have both a kernel TID and a PID

const KERNEL_STACK_SIZE_PER_THREAD: u64 = 2 * 4096; // 8KiB

const TICKS_PER_THREAD_SWITCH: usize = 20;

const MAX_TASKS: usize = 64;

#[repr(transparent)]
#[derive(PartialEq, Clone, Copy)]
pub struct ThreadID(usize);

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct RegisterState {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    rbp: u64,

    es: u64,
    ds: u64,
    fs: u64,
    gs: u64,

    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

impl RegisterState {
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
            rflags: (x86_64::Rflags::INTERRUPT | x86_64::Rflags::RESERVED_BIT_1).bits(),
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
            rflags: (x86_64::Rflags::INTERRUPT | x86_64::Rflags::RESERVED_BIT_1).bits(),
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

        write!(f, "rax: {:#x} rbx: {:#x} rcx: {:#x} rdx: {:#x} rsi: {:#x} rdi: {:#x} r8: {:#x} r9: {:#x} r10: {:#x} r11: {:#x} r12: {:#x} r13: {:#x} r14: {:#x} r15: {:#x} rbp: {:#x} es: {:#x} ds: {:#x} fs: {:#x} gs: {:#x} rip: {:#x} cs: {:#x} rflags: {:#x} rsp: {:#x} ss: {:#x}",
        rax, rbx, rcx, rdx, rsi, rdi, r8, r9, r10, r11, r12, r13, r14, r15, rbp,
        es, ds, fs, gs, rip, cs, rflags, rsp, ss)
    }
}

#[derive(Clone, Copy)]
pub enum ThreadState {
    None,
    Running,
    Busy,
}

pub struct Thread {
    id: ThreadID,
    state: ThreadState,
    user_thread: bool,
    stack_bottom: u64,
    kernel_regs: RegisterState,
    user_regs: RegisterState,
}

struct Scheduler {
    // what a resplendent piece of code
    threads: Vec<Option<Arc<Mutex<Thread>>>>,
    running_threads: Vec<ThreadID>,
    busy_threads: Vec<ThreadID>,
    current_queue: VecDeque<ThreadID>, // TID, front is always the current thread
    current_ticks: usize,
    thread_count: usize,
}

extern "C" {
    fn x86_64_switch_task(regs: RegisterState) -> !;
}

impl Scheduler {
    fn get_kernel_stack(tid: ThreadID) -> u64 {
        // FIXME: increase limit
        assert!(tid.0 < MAX_TASKS);
        KERNEL_THREAD_STACKS_START.get() + tid.0 as u64 * KERNEL_STACK_SIZE_PER_THREAD
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

    fn new_kernel_thread(&mut self) -> Thread {
        let tid = self.alloc_tid();
        Thread {
            id: tid,
            state: ThreadState::None,
            kernel_regs: RegisterState::kernel_new(),
            stack_bottom: Self::get_kernel_stack(tid) + KERNEL_STACK_SIZE_PER_THREAD,
            user_regs: RegisterState::user_new(),
            user_thread: false,
        }
    }

    /// spawns a kernel thread and returns the thread id
    fn spawn_kernel_thread(&mut self, func: fn()) -> Weak<Mutex<Thread>> {
        let tid: ThreadID;
        let thread = Arc::new(Mutex::new({
            let mut thread = self.new_kernel_thread();
            tid = thread.id;

            // push the address of remove_running_thread on the stack so the thread
            // will return to that address and get killed
            thread.kernel_regs.rsp = thread.stack_bottom;
            thread.kernel_regs.rsp -= core::mem::size_of::<u64>() as u64;
            unsafe {
                *(thread.kernel_regs.rsp as *mut u64) = remove_current_thread as u64;
            }

            thread.state = ThreadState::Running;
            thread.kernel_regs.rip = func as u64;
            thread
        }));

        let weak = Arc::downgrade(&thread);
        self.threads[tid.0] = Some(thread);
        self.running_threads.push(tid);

        println!("spawn_kernel_thread: {:#x}", tid.0);
        weak
    }

    fn new_user_thread(&mut self) -> Thread {
        let tid = self.alloc_tid();
        Thread {
            id: tid,
            state: ThreadState::None,
            stack_bottom: Self::get_kernel_stack(tid),
            kernel_regs: RegisterState::user_new(),
            user_regs: RegisterState::user_new(),
            user_thread: true,
        }
    }

    fn create_user_thread(&mut self) -> Weak<Mutex<Thread>> {
        let tid: ThreadID;
        let thread = Arc::new(Mutex::new({
            let thread = self.new_user_thread();
            tid = thread.id;
            thread
        }));

        let weak = Arc::downgrade(&thread);
        self.threads[tid.0] = Some(thread);

        println!("spawn_user_thread: {:#x}", tid.0);
        weak
    }

    /// saves the registers of the currently running thread
    fn save_regs(&mut self, regs: RegisterState) {
        let tid = *self.current_queue.front().unwrap();

        let thread_lock = self.get_thread(tid).unwrap();
        let mut thread = thread_lock.lock();
        // TODO: optimize this
        if thread.user_thread {
            thread.user_regs = regs;
        } else {
            thread.kernel_regs = regs;
        }
    }

    fn remove_from_current_queue(&mut self, tid: ThreadID) {
        let idx = self
            .current_queue
            .iter()
            .position(|thread_id| *thread_id == tid)
            .unwrap();
        self.current_queue.remove(idx);
    }

    fn remove_from_running_threads(&mut self, tid: ThreadID) {
        let idx = self
            .running_threads
            .iter()
            .position(|thread_id| *thread_id == tid)
            .unwrap();
        self.running_threads.remove(idx);
    }

    fn remove_from_busy_threads(&mut self, tid: ThreadID) {
        let idx = self
            .busy_threads
            .iter()
            .position(|thread_id| *thread_id == tid)
            .unwrap();
        self.busy_threads.remove(idx);
    }

    fn remove_thread(&mut self, tid: ThreadID) {
        // check whether we are removing the current thread
        assert!(*self.current_queue.front().unwrap() != tid);
        assert!(self.threads[tid.0].is_some());

        self.remove_from_current_queue(tid);

        let state = self.threads[tid.0].as_ref().unwrap().lock().state;
        match state {
            ThreadState::Busy => self.remove_from_busy_threads(tid),
            ThreadState::Running => self.remove_from_running_threads(tid),
            _ => unreachable!(),
        };

        self.thread_count -= 1;
        self.threads[tid.0] = None;
    }

    fn block_thread(&mut self, tid: ThreadID) {
        assert!(self.current_queue.len() != 0);

        // check whether we are removing the current thread
        let is_current_thread = *self.current_queue.front().unwrap() == tid;

        // find the thread and remove it
        let current_queue_idx = self
            .current_queue
            .iter()
            .position(|thread| *thread == tid)
            .unwrap();

        self.current_queue.remove(current_queue_idx);

        // move the thread from the running thread to the busy threads
        let running_threads_idx = self
            .running_threads
            .iter()
            .position(|thread_id| *thread_id == tid)
            .unwrap();

        self.running_threads.remove(running_threads_idx);
        self.busy_threads.push(tid);

        {
            let thread_lock = self.threads[tid.0].clone().unwrap();
            thread_lock.lock().state = ThreadState::Busy;
        }

        if is_current_thread {
            self.switch_thread();
        }
    }

    fn block_current_thread(&mut self) {
        let tid = *self.current_queue.front().unwrap();
        self.block_thread(tid);
    }

    /// removes current thread and switches to the next one
    fn remove_current_thread(&mut self) -> ! {
        let removed_tid = self.current_queue.pop_front().unwrap();
        // TODO: simply iterating over it may be faster
        let idx = self
            .running_threads
            .iter()
            .position(|tid| *tid == removed_tid)
            .unwrap();

        self.running_threads.remove(idx);
        self.threads[removed_tid.0] = None;

        self.switch_thread();
    }

    // such a splendiferous function
    fn get_thread(&mut self, tid: ThreadID) -> Option<Arc<Mutex<Thread>>> {
        let thread = self.threads.iter().find(|thread| match thread {
            Some(t) => t.lock().id == tid,
            None => false,
        });

        match thread {
            Some(t) => Some(Arc::clone(&t.as_ref().unwrap())),
            None => None,
        }
    }

    fn run_user_thread(&mut self, tid: ThreadID) {
        // TODO: add checks
        let lock = self.threads[tid.0].as_mut().unwrap();
        lock.lock().state = ThreadState::Running;
        self.running_threads.push(tid);
    }

    fn fill_queue(&mut self) {
        assert!(self.running_threads.len() != 0, "Sentinel is not running");

        // if no other threads are running add the sentinel thread to the queue
        if self.running_threads.len() == 1 {
            self.current_queue.push_back(ThreadID(0));
        }

        // otherwise add all running threads except the sentinel thread
        for tid in self.running_threads.iter().skip(1) {
            self.current_queue.push_back(*tid);
        }
    }

    /// this function SHOULD only be called from an interrupt or a thread that is
    /// about to be removed because the register state needs to be saved
    fn switch_thread(&mut self) -> ! {
        // TODO: save previous thread state

        // pop off the current thread
        // if this is none it means the front thread has been removed
        let _current_thread_id = self.current_queue.pop_front();

        // if the queue is empty start at the front of the running threads
        if self.current_queue.len() == 0 {
            self.fill_queue();
        }

        // the next thread because its always the current thread
        let next_thread_id = *self.current_queue.front().unwrap();
        let regs = {
            let next_thread_lock = self.get_thread(next_thread_id).unwrap();
            let next_thread = next_thread_lock.lock();
            unsafe {
                x86_64::tss::TSS.rsp0 = next_thread.stack_bottom;
            }
            next_thread.kernel_regs
        };

        if SCHEDULER.is_locked() {
            // we have to force unlock it because we wont return
            unsafe {
                SCHEDULER.force_unlock();
            }
        }

        //println!("switch thread: {}", next_thread_id.0);

        unsafe {
            // push the registers on the stack and switch tasks
            x86_64_switch_task(regs);
        }
    }

    pub fn tick(&mut self) {
        self.current_ticks += 1;
        if self.current_ticks < TICKS_PER_THREAD_SWITCH {
            return;
        }
        self.current_ticks = 0;
        self.switch_thread();
    }

    const fn new() -> Scheduler {
        Scheduler {
            threads: Vec::new(),
            running_threads: Vec::new(),
            busy_threads: Vec::new(),
            current_queue: VecDeque::new(),
            current_ticks: 0,
            thread_count: 0,
        }
    }
}

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub fn init() {
    // interrupts should be disasbled when this is called
    let mut sched = SCHEDULER.lock();

    const ALLOC_AT_ONCE: usize = 128;
    let kernel_stack_space_size = MAX_TASKS as u64 * KERNEL_STACK_SIZE_PER_THREAD;
    let in_pages = kernel_stack_space_size / 4096;

    assert!(in_pages % ALLOC_AT_ONCE as u64 == 0);
    let allocs = in_pages as usize / ALLOC_AT_ONCE;

    for i in 0..allocs {
        let phys_start = phys::alloc_multiple(ALLOC_AT_ONCE);
        for j in 0..ALLOC_AT_ONCE {
            let phys = phys_start + PhysAddr::new(j as u64 * 4096);
            let virt =
                KERNEL_THREAD_STACKS_START + VirtAddr::new((i * ALLOC_AT_ONCE + j) as u64 * 4096);
            virt::map_4kib(
                virt,
                phys,
                PageFlags::PRESENT | PageFlags::READ_WRITE,
                false,
            );
        }
    }

    sched.threads.resize(16, None);

    // spawn sentinel thread
    sched.spawn_kernel_thread(|| loop {
        println!("sentinel thread");
        loop {
            x86_64::enable_interrupts();
            unsafe {
                asm!("hlt");
            }
            // halt
        }
    });
}

pub fn spawn_kernel_thread(func: fn()) -> Weak<Mutex<Thread>> {
    assert!(!x86_64::interrupts_enabled());

    let mut sched = SCHEDULER.lock();
    sched.spawn_kernel_thread(func)
}

pub fn create_user_thread() -> Weak<Mutex<Thread>> {
    let interrupts_enabled = x86_64::interrupts_enabled();
    if interrupts_enabled {
        x86_64::disable_interrupts();
    }

    let val = {
        let mut sched = SCHEDULER.lock();
        sched.create_user_thread()
    };

    if interrupts_enabled {
        x86_64::enable_interrupts();
    }

    val
}

pub fn run_user_thread(tid: ThreadID) {
    let interrupts_enabled = x86_64::interrupts_enabled();
    if interrupts_enabled {
        x86_64::disable_interrupts();
    }

    {
        let mut sched = SCHEDULER.lock();
        sched.run_user_thread(tid);
    }

    if interrupts_enabled {
        x86_64::enable_interrupts();
    }
}

pub fn switch_thread() {
    assert!(!x86_64::interrupts_enabled());
    let mut sched = SCHEDULER.lock();
    sched.switch_thread();
}

pub fn tick() {
    assert!(!x86_64::interrupts_enabled());
    let mut sched = SCHEDULER.lock();
    sched.tick();
}

pub fn start() -> ! {
    let mut sched = SCHEDULER.lock();

    println!("scheduler start");

    // fill the queue
    sched.fill_queue();
    sched.switch_thread();
}

pub fn remove_current_thread() {
    x86_64::disable_interrupts();

    let mut sched = SCHEDULER.lock();
    sched.remove_current_thread();
}

pub fn current_tid() -> ThreadID {
    let sched = SCHEDULER.lock();
    *sched.current_queue.front().unwrap()
}

#[no_mangle]
extern "C" fn __block_current_thread() {
    let mut sched = SCHEDULER.lock();
    sched.block_current_thread();
}

pub fn block_current_thread() {
    x86_64::disable_interrupts();
    unsafe {
        x86_64::block_task();
    }
}

#[no_mangle]
pub extern "C" fn save_regs(regs: RegisterState) {
    assert!(!x86_64::interrupts_enabled());
    let mut sched = SCHEDULER.lock();
    sched.save_regs(regs);
}

#[no_mangle]
pub fn ticks_until_switch() -> u64 {
    assert!(!x86_64::interrupts_enabled());
    let val = {
        let sched = SCHEDULER.lock();
        (TICKS_PER_THREAD_SWITCH - sched.current_ticks) as u64
    };
    val
}
