pub mod proc;
pub mod queue;
pub mod thread;

use crate::{
    arch::x86_64::{
        self, disable_interrupts,
        registers::{InterruptRegisters, RegisterState},
        set_segment_selectors,
    },
    mm::virt::PML4,
    scheduler::thread::ThreadState,
    sync::InterruptMutex,
};

use core::arch::asm;

use alloc::sync::{Arc, Weak};
use spin::Mutex;

use self::{
    queue::SchedulerThreadQueue,
    thread::{SchedulerThreadData, Thread, ThreadID, ThreadInner},
};

// kernel thread IDs in the kernel are different from the PIDs of processes/threads
// a thread may have both a kernel TID and a PID

const TICKS_PER_THREAD_SWITCH: usize = 20;

pub struct Scheduler {
    thread_data: InterruptMutex<SchedulerThreadData>,
    queue: InterruptMutex<SchedulerThreadQueue>,
    ticks: InterruptMutex<usize>,
}

pub static SCHEDULER: Scheduler = Scheduler::new();

extern "C" {
    fn x86_64_switch_task(res: *const RegisterState) -> !;
}

impl Scheduler {
    fn remove_thread(&self, tid: ThreadID) {
        let mut queue = self.queue.lock();
        let mut thread_data = self.thread_data.lock();

        // check whether we are removing the current thread
        // TODO: only lock once
        assert!(*queue.front().unwrap() != tid);

        queue.remove_thread(tid);
        thread_data.remove_thread(tid);
    }

    fn block_thread(&self, tid: ThreadID) {
        // we encapsulate the locks in a block so switching thread won't
        // cause a deadlock
        let is_current_thread: bool;
        {
            let mut queue = self.queue.lock();
            let mut thread_data = self.thread_data.lock();

            is_current_thread = {
                let current_tid = *queue.front().expect("Thread queue is empty");
                current_tid == tid
            };

            queue.remove_thread(tid);
            thread_data.change_thread_state(tid, ThreadState::Busy);
        }

        if is_current_thread {
            self.force_switch_thread();
        }
    }

    pub fn block_current_thread(&self) {
        let tid = *self.queue.lock().front().unwrap();
        self.block_thread(tid);
    }

    pub fn get_current_thread(&self) -> Option<Arc<Mutex<Thread>>> {
        match self.queue.lock().front() {
            Some(&tid) => self.thread_data.lock().get_thread(tid),
            None => None,
        }
    }

    fn save_current_thread_regs(&self, int_regs: &InterruptRegisters) {
        let current_thread = match self.get_current_thread() {
            Some(thread) => thread,
            None => return,
        };

        let mut current_thread = current_thread.lock();

        // selectors don't change so there's no need to store them
        match &mut current_thread.inner {
            ThreadInner::Kernel(data) => {
                data.regs.general = int_regs.general;
                data.regs.rip = int_regs.iret.rip;
                data.regs.rsp = int_regs.iret.rsp;
            }
            ThreadInner::User(data) => {
                let regs = if data.in_kernelspace {
                    &mut data.kernel_regs
                } else {
                    &mut data.user_regs
                };

                regs.general = int_regs.general;
                regs.rip = int_regs.iret.rip;
                regs.rsp = int_regs.iret.rsp;
            }
        };
    }

    /// Removes current thread and switches to the next one
    pub fn remove_current_thread(&self) -> ! {
        // we encapsulate the locks in a block so switching thread won't
        // cause a deadlock
        {
            let mut queue = self.queue.lock();
            let mut thread_data = self.thread_data.lock();

            let tid = queue.pop_front().expect("Thread queue is empty");

            thread_data.remove_thread(tid);
        }

        self.force_switch_thread();
    }

    pub fn run_thread(&self, tid: ThreadID) {
        let mut thread_data = self.thread_data.lock();
        thread_data.change_thread_state(tid, ThreadState::Running);
    }

    fn next_thread(&self) -> Arc<Mutex<Thread>> {
        let mut queue = self.queue.lock();
        let thread_data = self.thread_data.lock();

        if !queue.is_empty() {
            // pop off the current thread
            // if this is none it means the front thread has been removed
            queue.pop_front().expect("Thread queue is empty");
        }

        // if the queue is empty start at the front of the running threads
        if queue.is_empty() {
            match thread_data.running_threads.len() {
                0 => panic!("Sentinel is not running"),
                // if no other threads are running add the sentinel thread to the queue
                1 => queue.add_thread(ThreadID(0)),
                // otherwise add all running threads except the sentinel thread
                _ => thread_data
                    .running_threads
                    .iter()
                    .skip(1)
                    .for_each(|&tid| queue.add_thread(tid)),
            };
        }

        let next_thread_id = *queue.front().expect("Thread queue is empty");
        thread_data
            .get_thread(next_thread_id)
            .expect("Invalid next thread id")
    }

    /// this function should only be called from a thread that is about to be removed or blocked
    fn force_switch_thread(&self) -> ! {
        disable_interrupts();

        // we encapsulate the locks in a block so switching thread won't
        // cause a deadlock
        let regs = {
            let next_thread = self.next_thread();
            let next_thread = next_thread.lock();

            unsafe {
                x86_64::tss::TSS.rsp0 = next_thread.stack_bottom;
            }

            let regs = match &next_thread.inner {
                ThreadInner::Kernel(data) => &data.regs,
                ThreadInner::User(data) => {
                    if data.in_kernelspace {
                        &data.kernel_regs
                    } else {
                        &data.user_regs
                    }
                }
            };

            **regs
        };

        // TODO: dont copy registers
        unsafe {
            let ptr = &regs as *const RegisterState;
            // push the registers on the stack and switch tasks
            x86_64_switch_task(ptr);
        }
    }

    pub fn tick(&self, int_regs: &mut InterruptRegisters) {
        //println!("tick");
        {
            let mut ticks = self.ticks.lock();
            *ticks += 1;
            if *ticks < TICKS_PER_THREAD_SWITCH {
                return;
            }

            *ticks = 0;
        }

        self.save_current_thread_regs(int_regs);

        let next_thread = self.next_thread();
        let next_thread = next_thread.lock();

        //println!("switch thread {}", next_thread.id.0);

        // TODO: dont copy registers
        let regs = match &next_thread.inner {
            ThreadInner::Kernel(data) => &data.regs,
            ThreadInner::User(data) => {
                if data.in_kernelspace {
                    &data.kernel_regs
                } else {
                    &data.user_regs
                }
            }
        };

        set_segment_selectors(regs.selectors.es);

        int_regs.general = regs.general;
        int_regs.iret.rip = regs.rip;
        int_regs.iret.rsp = regs.rsp;
        int_regs.iret.ss = regs.selectors.ss;
        int_regs.iret.cs = regs.selectors.cs;
        int_regs.iret.rflags = regs.rflags;
    }

    pub fn start(&self) -> ! {
        self.force_switch_thread();
    }

    pub fn init(&self, pml4: &PML4) {
        let mut thread_data = self.thread_data.lock();
        thread_data.init(pml4);

        // spawn sentinel thread
        thread_data.create_kernel_thread(|| loop {
            debug!("in sentinel thread");
            loop {
                x86_64::enable_interrupts();
                unsafe {
                    asm!("hlt");
                }
                // halt
            }
        });
    }

    pub fn create_user_thread(&self, pid: usize) -> Weak<Mutex<Thread>> {
        let mut thread_data = self.thread_data.lock();
        thread_data.create_user_thread(pid)
    }

    pub fn create_kernel_thread(&self, f: fn()) -> Weak<Mutex<Thread>> {
        let mut thread_data = self.thread_data.lock();
        thread_data.create_kernel_thread(f)
    }

    pub fn copy_user_thread(&self, pid: usize, tid: ThreadID) -> Weak<Mutex<Thread>> {
        let mut thread_data = self.thread_data.lock();
        thread_data.copy_user_thread(pid, tid)
    }

    pub fn ticks(&self) -> usize {
        *self.ticks.lock()
    }

    const fn new() -> Self {
        Scheduler {
            thread_data: InterruptMutex::new(SchedulerThreadData::new()),
            queue: InterruptMutex::new(SchedulerThreadQueue::new()),
            ticks: InterruptMutex::new(0),
        }
    }
}

pub fn remove_current_thread_wrapper() {
    SCHEDULER.remove_current_thread();
}

#[no_mangle]
extern "C" fn __block_current_thread() {
    SCHEDULER.block_current_thread();
}
