use alloc::vec::Vec;
use spin::Mutex;

use crate::fs::{self, FileSystemError};

use super::Thread;

pub struct Process {
    pid: usize,
    main_thread: Thread,
}

static PROCESSES: Mutex<Vec<Option<Process>>> = Mutex::new(Vec::new());

impl Process {
    fn new() -> Process {
        let pid = get_new_pid();

        Process {
            pid,
            main_thread: Thread::new_kernel_thread(),
        }
    }
}

fn get_new_pid() -> usize {
    let mut processes = PROCESSES.lock();
    let pid = match processes.iter().position(|x| x.is_none()) {
        Some(x) => x,
        None => {
            let old_len = processes.len();
            processes.resize_with(old_len * 2, || None);
            old_len
        }
    } + 1;

    pid
}

pub fn load_process(path: &str) {
    let fd = fs::open(path).unwrap();
    let info = fd.file_info().unwrap();
    println!("{} {}", info.size, info.blocks_used);

    let proc = Process::new();
    println!("PID {}", proc.pid);
}
