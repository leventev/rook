use alloc::{boxed::Box, slice, vec::Vec};
use elf::{
    abi::{PF_W, PT_LOAD},
    endian::LittleEndian,
    ElfBytes,
};
use spin::Mutex;

use crate::{
    arch::x86_64::{get_current_pml4, paging::PageFlags},
    fs,
    mm::{
        phys,
        virt::{self, map_4kib_other, switch_pml4, PAGE_SIZE_4KIB},
        PhysAddr, VirtAddr,
    },
    utils,
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

pub fn load_process(proc: &mut Process, exec_path: &str) -> bool {
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

    // TODO: check if the segments are in userspace
    for ph in segments {
        let pages = utils::div_and_ceil(ph.p_filesz as usize, PAGE_SIZE_4KIB as usize);
        let seg_start = VirtAddr::new(ph.p_vaddr - ph.p_vaddr % PAGE_SIZE_4KIB);
        for i in 0..pages {
            let phys = phys::alloc();
            let virt = VirtAddr::new(seg_start.get() + i as u64 * PAGE_SIZE_4KIB);

            let mut flags = PageFlags::PRESENT | PageFlags::READ_WRITE | PageFlags::USER;
            if ph.p_flags & PF_W > 0 {
                flags |= PageFlags::READ_WRITE;
            }

            map_4kib_other(proc.pml4_phys, virt, phys, flags, true);
        }

        switch_pml4(proc.pml4_phys);

        let file_seg_start = ph.p_offset as usize;
        let file_seg_end = (ph.p_offset + ph.p_filesz) as usize;

        let seg_mem =
            unsafe { slice::from_raw_parts_mut(seg_start.get() as *mut u8, ph.p_filesz as usize) };
        seg_mem.copy_from_slice(&buff[file_seg_start..file_seg_end]);

        println!("{:?}", ph);
        println!("{:#x}", ph.p_vaddr);
    }

    true
}

pub fn load_base_process(exec_path: &str) {
    let mut proc = Process::new();
    println!("PID {}", proc.pid);

    load_process(&mut proc, exec_path);
}
