use alloc::{boxed::Box, sync::Arc, sync::Weak, vec::Vec};
use spin::Mutex;

use crate::{
    arch::x86_64::{interrupts_enabled, paging::PageFlags, registers::RegisterState},
    mm::{
        phys::{self, FRAME_SIZE},
        virt::{KERNEL_THREAD_STACKS_START, PML4},
        VirtAddr,
    },
    scheduler::remove_current_thread_wrapper,
};

#[repr(transparent)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ThreadID(pub usize);

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

// FIXME: do not derive Clone because it won't allocate a new TLS
#[derive(Debug, Clone)]
pub struct UserThreadData {
    pub pid: usize,
    pub kernel_regs: Box<RegisterState>,
    pub user_regs: Box<RegisterState>,
    pub in_kernelspace: bool,
    pub tls: VirtAddr,
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

// we leave the lowest page of each thread stack space unmapped so a stackoverflow triggers a pagefault
const KERNEL_FULL_STACK_SIZE_PER_THREAD: u64 = 8 * 4096; // 32KiB
const KERNEL_STACK_SIZE_PER_THREAD: u64 = KERNEL_FULL_STACK_SIZE_PER_THREAD - 4096; // 28 KiB

const MAX_THREADS: usize = 64;

impl SchedulerThreadData {
    fn get_kernel_stack(tid: ThreadID) -> u64 {
        // FIXME: increase limit
        assert!(tid.0 < MAX_THREADS);
        KERNEL_THREAD_STACKS_START.get() + tid.0 as u64 * KERNEL_FULL_STACK_SIZE_PER_THREAD
    }

    pub fn init(&mut self, pml4: &PML4) {
        assert!(!interrupts_enabled());

        // TODO: allocate stacks on demand
        for tid in 0..MAX_THREADS {
            // skip the first one
            let thread_stack_bottom = VirtAddr::new(Self::get_kernel_stack(ThreadID(tid)));
            let in_pages = KERNEL_STACK_SIZE_PER_THREAD / FRAME_SIZE as u64;

            // leave first page unmapped so a stack overflow causes a pagefault
            for i in 1..=in_pages {
                let phys = phys::alloc();
                let virt = thread_stack_bottom + VirtAddr::new(i * FRAME_SIZE as u64);
                pml4.map_4kib(virt, phys, PageFlags::PRESENT | PageFlags::READ_WRITE);
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
                regs: Box::new(RegisterState::new_kernel()),
            }),
            stack_bottom: Self::get_kernel_stack(tid) + KERNEL_FULL_STACK_SIZE_PER_THREAD,
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

        debug!("spawn kernel thread: {:#x}", tid.0);
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
                kernel_regs: Box::new(RegisterState::new_kernel()),
                user_regs: Box::new(RegisterState::new_user()),
                in_kernelspace: false,
                tls: VirtAddr::new(0),
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

        debug!("spawn user thread: {:#x}", tid.0);
        weak
    }

    pub fn copy_user_thread(&mut self, pid: usize, tid: ThreadID) -> Weak<Mutex<Thread>> {
        let new_tid = self.alloc_tid();

        let new_thread = Arc::new(Mutex::new({
            let old_thread = self.threads[tid.0].as_ref().expect("Invalid TID");
            let old_thread = old_thread.lock();

            let mut thread = old_thread.clone();
            thread.id = new_tid;
            thread.state = ThreadState::None;

            if let ThreadInner::User(data) = &mut thread.inner {
                data.pid = pid;
            } else {
                unreachable!()
            }

            thread
        }));

        let weak = Arc::downgrade(&new_thread);
        self.threads[new_tid.0] = Some(new_thread);

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
        self.threads[tid.0].as_ref().cloned()
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
