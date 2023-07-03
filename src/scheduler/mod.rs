pub mod proc;
pub mod queue;
pub mod thread;

use crate::{arch::x86_64::{self, disable_interrupts}, scheduler::thread::ThreadState, sync::InterruptMutex};

use core::arch::asm;

use alloc::sync::{Arc, Weak};
use spin::Mutex;

use self::{
    queue::SchedulerThreadQueue,
    thread::{RegisterState, SchedulerThreadData, Thread, ThreadID, ThreadInner},
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
    fn x86_64_switch_task(regs: RegisterState) -> !;
}

impl Scheduler {
    /// saves the registers of the currently running thread
    fn save_regs(&self, registers: RegisterState) {
        let queue = self.queue.lock();
        let thread_data = self.thread_data.lock();

        if queue.is_empty() {
            return;
        }

        let tid = *queue.front().unwrap();

        let thread_lock = thread_data.get_thread(tid).unwrap();
        let mut thread = thread_lock.lock();

        let reg_ref = match &mut thread.inner {
            ThreadInner::Kernel(data) => &mut data.regs,
            ThreadInner::User(data) => {
                if data.in_kernelspace {
                    &mut data.kernel_regs
                } else {
                    &mut data.user_regs
                }
            }
        };
        **reg_ref = registers;
    }

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
            thread_data.change_thread_state(tid, ThreadState::Running);
        }

        if is_current_thread {
            self.switch_thread();
        }
    }

    fn block_current_thread(&self) {
        let tid = *self.queue.lock().front().unwrap();
        self.block_thread(tid);
    }

    pub fn get_current_thread(&self) -> Arc<Mutex<Thread>> {
        let tid = *self.queue.lock().front().unwrap();
        self.thread_data.lock().get_thread(tid).unwrap()
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

        self.switch_thread();
    }

    pub fn run_thread(&self, tid: ThreadID) {
        let mut thread_data = self.thread_data.lock();
        thread_data.change_thread_state(tid, ThreadState::Running);
    }

    /// this function SHOULD only be called from an interrupt or a thread that is
    /// about to be removed because the register state needs to be saved
    fn switch_thread(&self) -> ! {
        // TODO: save previous thread state

        // we encapsulate the locks in a block so switching thread won't
        // cause a deadlock
        let regs = {
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

            // the next thread because its always the current thread
            let next_thread_id = *queue.front().expect("Thread queue is empty");
            let next_thread = thread_data
                .get_thread(next_thread_id)
                .expect("Invalid next thread id");

            let next_thread = next_thread.lock();

            unsafe {
                x86_64::tss::TSS.rsp0 = next_thread.stack_bottom;
            }

            //println!("switch thread: {}", next_thread_id.0);

            match &next_thread.inner {
                ThreadInner::Kernel(data) => *data.regs,
                ThreadInner::User(data) => {
                    if data.in_kernelspace {
                        *data.kernel_regs
                    } else {
                        *data.user_regs
                    }
                }
            }
        };

        unsafe {
            // push the registers on the stack and switch tasks
            x86_64_switch_task(regs);
        }
    }

    pub fn tick(&self) {
        {
            let mut ticks = self.ticks.lock();
            *ticks += 1;
            if *ticks < TICKS_PER_THREAD_SWITCH {
                return;
            }

            *ticks = 0;
        }
        self.switch_thread();
    }

    pub fn start(&self) -> ! {
        self.switch_thread();
    }

    pub fn init(&self) {
        let mut thread_data = self.thread_data.lock();
        thread_data.init();

        // spawn sentinel thread
        thread_data.create_kernel_thread(|| loop {
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

    pub fn create_user_thread(&self, pid: usize) -> Weak<Mutex<Thread>> {
        let mut thread_data = self.thread_data.lock();
        thread_data.create_user_thread(pid)
    }

    pub fn create_kernel_thread(&self, f: fn()) -> Weak<Mutex<Thread>> {
        let mut thread_data = self.thread_data.lock();
        thread_data.create_kernel_thread(f)
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

#[no_mangle]
pub extern "C" fn save_regs(regs: RegisterState) {
    assert!(!x86_64::interrupts_enabled());
    SCHEDULER.save_regs(regs);
}

#[no_mangle]
pub fn ticks_until_switch() -> u64 {
    assert!(!x86_64::interrupts_enabled());
    let ticks = SCHEDULER.ticks();
    (TICKS_PER_THREAD_SWITCH - ticks) as u64
}
