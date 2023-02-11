use alloc::{boxed::Box, vec::Vec};
use elf::{abi::PT_LOAD, endian::LittleEndian, ElfBytes};
use spin::Mutex;

use crate::{
    fs,
    mm::{phys, virt, PhysAddr}, arch::x86_64::get_current_pml4,
};

use super::Thread;

pub struct Process {
    pid: usize,
    main_thread: Thread,
    pml4_phys: PhysAddr,
}

static PROCESSES: Mutex<Vec<Option<Process>>> = Mutex::new(Vec::new());

impl Process {
    fn new() -> Process {
        let pid = get_new_pid();

        let pml4 = phys::alloc();
        virt::copy_pml4_higher_half_entries(pml4, get_current_pml4());

        Process {
            pid,
            main_thread: Thread::new_kernel_thread(),
            pml4_phys: pml4,
        }
    }
}

fn get_new_pid() -> usize {
    let mut processes = PROCESSES.lock();
    let pid = match processes.iter().position(Option::is_none) {
        Some(x) => x,
        None => {
            let old_len = processes.len();
            processes.resize_with(old_len * 2, || None);
            old_len
        }
    } + 1;

    pid
}

pub fn load_process(_proc: &mut Process, exec_path: &str) -> bool {
    let mut fd = fs::open(exec_path).unwrap();
    let info = fd.file_info().unwrap();
    println!("{} {}", info.size, info.blocks_used);

    // TODO: perhaps we can parse the ELF header without reading the whole file
    // and instead later reading the file to userspace
    // TODO: don't unnecessarily zero the memory
    let mut buff: Box<[u8]> = vec![0; info.size].into_boxed_slice();
    println!("buff.len: {} info.size: {}", buff.len(), info.size);
    if fd.read(info.size, &mut buff[..]).is_err() {
        return false;
    }

    let elf_file = match ElfBytes::<LittleEndian>::minimal_parse(&buff[..]) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let segments = match elf_file.segments() {
        Some(segs) => segs,
        None => return false,
    }
    .iter()
    .filter(|seg| seg.p_type == PT_LOAD);

    for ph in segments {
        println!("{:?}", ph);
    }

    true
}

pub fn load_base_process(exec_path: &str) {
    let mut proc = Process::new();
    println!("PID {}", proc.pid);

    load_process(&mut proc, exec_path);
}
