pub mod proc;

use crate::{
    arch::x86_64::{self, paging::PageFlags},
    mm::{phys, virt, PhysAddr, VirtAddr},
};
use core::arch::asm;

use core::fmt;

use alloc::{collections::VecDeque, vec::Vec};
use spin::Mutex;

// kernel thread IDs in the kernel are different from the PIDs of processes/threads
// a thread may have both a kernel TID and a PID

// pml4[508]
const KERNEL_THREAD_STACKS_START: VirtAddr = VirtAddr::new(0xfffffe0000000000);
const KERNEL_STACK_SIZE_PER_THREAD: u64 = 2 * 4096; // 8KiB

const TICKS_PER_THREAD_SWITCH: usize = 20;

const MAX_TASKS: usize = 64;

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
            es: 0x30,
            ds: 0x30,
            fs: 0x30,
            gs: 0x30,
            rip: 0,
            cs: 0x28,
            rflags: (x86_64::Rflags::INTERRUPT | x86_64::Rflags::RESERVED_BIT_1).bits(),
            // FIXME: this may not be the best option ^^^^
            rsp: 0,
            ss: 0x30,
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
            es: 0x30,
            ds: 0x30,
            fs: 0x30,
            gs: 0x30,
            rip: 0,
            cs: 0x28,
            rflags: (x86_64::Rflags::INTERRUPT | x86_64::Rflags::RESERVED_BIT_1).bits(),
            // FIXME: this may not be the best option ^^^^
            rsp: 0,
            ss: 0x30,
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

pub struct Thread {
    id: usize,
    regs: RegisterState,
}

impl Thread {
    fn new_kernel_thread() -> Thread {
        Thread {
            id: 0,
            regs: RegisterState::kernel_new(),
        }
    }

    fn new_user_thread() -> Thread {
        Thread {
            id: 0,
            regs: RegisterState::user_new(),
        }
    }
}

struct Scheduler {
    running_threads: Vec<Thread>,
    busy_threads: Vec<Thread>,
    current_queue: VecDeque<usize>, // TID, front is always the current thread
    current_ticks: usize,
    temp_counter: usize,
}

extern "C" {
    fn x86_64_switch_task(regs: RegisterState) -> !;
}

impl Scheduler {
    fn alloc_kernel_stack(tid: usize) -> u64 {
        // FIXME: increase limit
        assert!(tid < MAX_TASKS);
        KERNEL_THREAD_STACKS_START.get() + tid as u64 * KERNEL_STACK_SIZE_PER_THREAD
    }

    /// spawns a kernel thread and returns the thread id
    fn spawn_kernel_thread(&mut self, func: fn()) -> usize {
        let mut thread = Thread::new_kernel_thread();
        let tid = self.temp_counter;
        self.temp_counter += 1;

        thread.id = tid;
        thread.regs.rsp = Scheduler::alloc_kernel_stack(thread.id) + KERNEL_STACK_SIZE_PER_THREAD;

        // push the address of remove_running_thread on the stack so the thread
        // will return to that address and get killed

        thread.regs.rsp -= core::mem::size_of::<u64>() as u64;
        unsafe {
            *(thread.regs.rsp as *mut u64) = remove_running_thread as u64;
        }

        thread.regs.rip = func as u64;

        println!("spawn_kernel_thread: {:#x}", func as u64);
        self.running_threads.push(thread);

        tid
    }

    /// saves the registers of the currently running thread
    fn save_regs(&mut self, regs: RegisterState) {
        // println!("{:?}", regs);
        let tid = *self.current_queue.front().unwrap();
        // TODO: simply iterating over it may be faster
        let thread = self.get_running_thread(tid).unwrap();
        thread.regs = regs;
    }

    /// returns whethere the linked list contained the TID or not
    fn remove_running_thread(&mut self, tid: usize) -> bool {
        if self.current_queue.len() == 0 {
            return false;
        }

        // check whether we are removing the current thread
        assert!(*self.current_queue.front().unwrap() != tid);

        // find the thread and remove it
        for (idx, &thread) in self.current_queue.iter().enumerate() {
            if thread != tid {
                continue;
            }
            self.current_queue.remove(idx);
            return true;
        }

        false
    }

    fn block_thread(&mut self, tid: usize) {
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
            .position(|thread| thread.id == tid)
            .unwrap();

        let thread = self.running_threads.remove(running_threads_idx);
        self.busy_threads.push(thread);

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
        let removed_thread = self.current_queue.pop_front().unwrap();
        // TODO: simply iterating over it may be faster
        let idx = self
            .running_threads
            .iter()
            .position(|thread| thread.id == removed_thread)
            .unwrap();

        self.running_threads.remove(idx);

        self.switch_thread();
    }

    fn get_running_thread(&mut self, tid: usize) -> Option<&mut Thread> {
        self.running_threads
            .iter_mut()
            .find(|thread| thread.id == tid)
    }

    fn fill_queue(&mut self) {
        assert!(self.running_threads.len() != 0, "Sentinel is not running");

        // if no other threads are running add the sentinel thread to the queue
        if self.running_threads.len() == 1 {
            self.current_queue.push_back(0);
        }

        // otherwise add all running threads except the sentinel thread
        for thread in self.running_threads.iter().skip(1) {
            self.current_queue.push_back(thread.id);
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
        let next_thread = self.get_running_thread(next_thread_id).unwrap();

        if SCHEDULER.is_locked() {
            // we have to force unlock it because we wont return
            unsafe {
                SCHEDULER.force_unlock();
            }
        }

        //println!("switch thread: {}", next_thread_id);

        unsafe {
            // push the registers on the stack and switch tasks
            x86_64_switch_task(next_thread.regs);
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
            running_threads: Vec::new(),
            busy_threads: Vec::new(),
            current_queue: VecDeque::new(),
            current_ticks: 0,
            temp_counter: 0,
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
            virt::map(virt, phys, PageFlags::READ_WRITE);
        }
    }

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

pub fn spawn_kernel_thread(func: fn()) {
    assert!(!x86_64::interrupts_enabled());
    let mut sched = SCHEDULER.lock();
    sched.spawn_kernel_thread(func);
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

pub fn remove_running_thread() {
    x86_64::disable_interrupts();

    let mut sched = SCHEDULER.lock();
    sched.remove_current_thread();
}

pub fn current_tid() -> usize {
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
