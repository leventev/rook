use alloc::collections::VecDeque;

use super::thread::ThreadID;

pub struct SchedulerThreadQueue {
    queue: VecDeque<ThreadID>,
}

impl SchedulerThreadQueue {
    pub fn front(&self) -> Option<&ThreadID> {
        self.queue.front()
    }

    pub fn pop_front(&mut self) -> Option<ThreadID> {
        self.queue.pop_front()
    }

    pub fn add_thread(&mut self, tid: ThreadID) {
        self.queue.push_back(tid);
    }

    pub fn remove_thread(&mut self, tid: ThreadID) {
        let idx = self
            .queue
            .iter()
            .position(|thread_id| *thread_id == tid)
            .unwrap();

        self.queue.remove(idx);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub const fn new() -> Self {
        SchedulerThreadQueue {
            queue: VecDeque::new(),
        }
    }
}
