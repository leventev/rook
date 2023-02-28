use crate::{
    arch::x86_64::{get_current_pml4, paging::PageFlags},
    fs::{self, FileDescriptor},
    mm::{
        phys,
        virt::{self, map_4kib, switch_pml4, PAGE_SIZE_4KIB},
        PhysAddr, VirtAddr,
    },
    posix::AT_FCWD,
    scheduler::{create_user_thread, run_user_thread},
    utils,
};
use alloc::{
    boxed::Box,
    slice,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use elf::{
    abi::{PF_W, PT_LOAD},
    endian::LittleEndian,
    ElfBytes,
};
use spin::Mutex;

use super::Thread;

pub struct Process {
    pub pid: usize,
    main_thread: Weak<Mutex<Thread>>,
    pml4_phys: PhysAddr,
    file_descriptors: Vec<Option<Arc<Mutex<FileDescriptor>>>>,
    file_descriptors_allocated: usize,
}

unsafe impl Send for Process {}

static PROCESSES: Mutex<Vec<Option<Arc<Mutex<Process>>>>> = Mutex::new(Vec::new());

impl Process {
    fn new() -> Process {
        let pml4 = phys::alloc();
        virt::copy_pml4_higher_half_entries(pml4, get_current_pml4());

        Process {
            pid: 0,
            main_thread: create_user_thread(),
            pml4_phys: pml4,
            file_descriptors: Vec::with_capacity(8),
            file_descriptors_allocated: 0,
        }
    }

    pub fn new_fd(
        &mut self,
        hint: Option<usize>,
        file_descriptor: Arc<Mutex<FileDescriptor>>,
    ) -> Result<usize, ()> {
        assert!(self.file_descriptors_allocated <= self.file_descriptors.capacity());

        let fd = match hint {
            Some(n) => {
                // check if the hint is already in use
                if n < self.file_descriptors.len() {
                    if self.file_descriptors[n].is_some() {
                        return Err(());
                    }
                }

                // if a hint was provided allocate enough space then return
                let mut size = self.file_descriptors.capacity();
                while size < n {
                    size *= 2;
                }
                self.file_descriptors.resize_with(size, || None);
                n
            }
            None => {
                if self.file_descriptors_allocated == self.file_descriptors.capacity() {
                    // if there is not enough space, double the vector size
                    let old_size = self.file_descriptors.capacity();
                    let size = old_size * 2;
                    self.file_descriptors.resize_with(size, || None);

                    old_size
                } else {
                    // else find the first free fd
                    self.file_descriptors
                        .iter()
                        .position(Option::is_none)
                        .unwrap()
                }
            }
        };

        self.file_descriptors[fd] = Some(file_descriptor);
        self.file_descriptors_allocated += 1;
        Ok(fd)
    }

    // TODO: error
    pub fn dup_fd(&mut self, hint: Option<usize>, fd: usize) -> Result<usize, ()> {
        let file_desc = match self.file_descriptors[fd] {
            Some(ref f) => Arc::clone(f),
            None => return Err(()),
        };

        self.new_fd(hint, file_desc)
    }

    pub fn free_fd(&mut self, fd: usize) {
        assert!(fd < self.file_descriptors.capacity());
        assert!(self.file_descriptors[fd].is_some());
        self.file_descriptors[fd] = None;
    }

    pub fn get_fd(&self, fd: usize) -> Option<Arc<Mutex<FileDescriptor>>> {
        self.file_descriptors.get(fd).unwrap_or(&None).clone()
    }

    /// Only possible error is an invalid fd
    pub fn get_full_path_from_dirfd(&self, dirfd: isize, path: &str) -> Result<String, ()> {
        if path.starts_with('/') {
            // if the path is absolute we ignore the value of dirfd
            Ok(String::from(path))
        } else {
            if dirfd == AT_FCWD {
                todo!()
            } else if dirfd < 0 {
                return Err(());
            };

            let fd = dirfd as usize;
            let file_lock = match self.get_fd(fd) {
                Some(f) => f,
                None => return Err(()),
            };

            let file_desc = file_lock.lock();

            // TODO: faster way to use the base path
            let base_path = file_desc.vnode.path();
            Ok(format!("{}/{}", base_path, path))
        }
    }
}

fn add_process(mut proc: Process) -> usize {
    let mut processes = PROCESSES.lock();
    let pid = match processes.iter().position(Option::is_none) {
        Some(x) => x,
        None => {
            let old_len = processes.len();
            let new_len = if old_len == 0 { 8 } else { old_len * 2 };
            processes.resize_with(new_len, || None);
            old_len
        }
    } + 1;

    proc.pid = pid;
    processes[pid - 1] = Some(Arc::new(Mutex::new(proc)));

    pid
}

pub fn load_process(proc: &mut Process, exec_path: &str) -> bool {
    let main_thread_lock = proc.main_thread.upgrade().unwrap();
    let mut main_thread = main_thread_lock.lock();

    let mut fd = fs::open(exec_path).unwrap();
    let info = fd.file_info().unwrap();

    // TODO: perhaps we can parse the ELF header without reading the whole file
    // and instead later reading the file to userspace
    // TODO: don't unnecessarily zero the memory
    let mut buff: Box<[u8]> = vec![0; info.size].into_boxed_slice();
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

    switch_pml4(proc.pml4_phys);
    // TODO: check if the segments are in userspace
    for ph in segments {
        let pages = utils::div_and_ceil(ph.p_memsz as usize, PAGE_SIZE_4KIB as usize);
        let seg_page_start = VirtAddr::new(ph.p_vaddr - ph.p_vaddr % PAGE_SIZE_4KIB);
        for i in 0..pages {
            let phys = phys::alloc();
            let virt = VirtAddr::new(seg_page_start.get() + i as u64 * PAGE_SIZE_4KIB);

            let mut flags = PageFlags::PRESENT | PageFlags::READ_WRITE | PageFlags::USER;
            if ph.p_flags & PF_W > 0 {
                flags |= PageFlags::READ_WRITE;
            }

            map_4kib(virt, phys, flags, true);
        }

        let file_seg_start = ph.p_offset as usize;
        let file_seg_end = (ph.p_offset + ph.p_filesz) as usize;

        let file_seg_mem =
            unsafe { slice::from_raw_parts_mut(ph.p_vaddr as *mut u8, ph.p_filesz as usize) };
        file_seg_mem.copy_from_slice(&buff[file_seg_start..file_seg_end]);

        if ph.p_memsz != ph.p_filesz {
            let mem_file_size_diff = (ph.p_memsz - ph.p_filesz) as usize;
            let start = (ph.p_vaddr + ph.p_filesz) as *mut u8;

            let seg_mem = unsafe { slice::from_raw_parts_mut(start, mem_file_size_diff) };
            for i in 0..mem_file_size_diff {
                seg_mem[i] = 0;
            }
        }
    }

    const STACK_BASE: u64 = 0xfffffd8000000000;
    const STACK_SIZE: u64 = 4096 * 16; // 64 KiB
    let pages = STACK_SIZE / 4096;
    for i in 0..pages {
        let virt = VirtAddr::new(STACK_BASE + i * 4096);
        let phys = phys::alloc();
        map_4kib(
            virt,
            phys,
            PageFlags::READ_WRITE | PageFlags::USER | PageFlags::PRESENT,
            false,
        );
    }

    let stack_bottom = STACK_BASE + STACK_SIZE;

    // TODO: validate
    main_thread.user_regs.rip = elf_file.ehdr.e_entry;
    main_thread.user_regs.rsp = stack_bottom;
    main_thread.process_id = proc.pid;

    true
}

pub fn load_base_process(exec_path: &str) {
    let pid = add_process(Process::new());
    let proc_lock = get_process(pid).unwrap();
    let mut proc = proc_lock.lock();

    // open console
    let console_fd = fs::open("/dev/console").expect("Failed to open /dev/console");

    // stdin
    let fd = proc
        .new_fd(Some(0), Arc::new(Mutex::new(*console_fd)))
        .unwrap();
    assert!(fd == 0);

    // stdout
    let fd = proc.dup_fd(None, fd).unwrap();
    assert!(fd == 1);

    // stderr
    let fd = proc.dup_fd(None, fd).unwrap();
    assert!(fd == 2);

    let main_thread_id = { proc.main_thread.upgrade().unwrap().lock().id };

    if !load_process(&mut proc, exec_path) {
        panic!("failed to load base process");
    }

    run_user_thread(main_thread_id);
}

pub fn get_process(pid: usize) -> Option<Arc<Mutex<Process>>> {
    let processes = PROCESSES.lock();
    processes[pid - 1].clone()
}
