use crate::{
    arch::x86_64::{get_current_pml4, paging::PageFlags},
    fs::{self, FileDescriptor},
    mm::{
        phys,
        virt::{self, switch_pml4, PAGE_SIZE_4KIB},
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
    abi::{PF_X, PT_LOAD},
    endian::LittleEndian,
    ElfBytes,
};
use spin::Mutex;

use super::Thread;

bitflags::bitflags! {
    pub struct MappedRegionFlags: u64 {
        const READ_WRITE = 1 << 0;
        const ALLOC_ON_ACCESS = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

#[derive(Debug)]
pub struct MappedRegion {
    start: usize,
    pages: usize,
    end: usize,
    flags: MappedRegionFlags,
}

impl MappedRegion {
    const fn new(start: usize, pages: usize, flags: MappedRegionFlags) -> MappedRegion {
        MappedRegion {
            start,
            pages,
            end: start + pages * PAGE_SIZE_4KIB as usize,
            flags,
        }
    }

    fn page_flags(&self) -> PageFlags {
        let mut flags = PageFlags::USER;
        if self.flags.contains(MappedRegionFlags::READ_WRITE) {
            flags |= PageFlags::READ_WRITE;
        }

        if self.flags.contains(MappedRegionFlags::ALLOC_ON_ACCESS) {
            flags |= PageFlags::ALLOC_ON_ACCESS;
        } else {
            flags |= PageFlags::PRESENT;
        }

        flags
    }

    fn virt_addr(&self) -> VirtAddr {
        VirtAddr::new(self.start as u64)
    }
}

pub struct Process {
    pub pid: usize,
    pub ppid: usize,
    pub pgid: usize,

    pub uid: usize,
    pub euid: usize,
    pub gid: usize,
    pub egid: usize,

    pub cwd: Arc<Mutex<FileDescriptor>>,

    mapped_regions: Vec<MappedRegion>,

    pub main_thread: Weak<Mutex<Thread>>,
    pml4_phys: PhysAddr,
    file_descriptors: Vec<Option<Arc<Mutex<FileDescriptor>>>>,
    file_descriptors_allocated: usize,
}

unsafe impl Send for Process {}

static PROCESSES: Mutex<Vec<Option<Arc<Mutex<Process>>>>> = Mutex::new(Vec::new());

impl Process {
    fn new(
        uid: usize,
        euid: usize,
        gid: usize,
        egid: usize,
        cwd: Arc<Mutex<FileDescriptor>>,
    ) -> Process {
        let pml4 = phys::alloc();
        virt::copy_pml4_higher_half_entries(pml4, get_current_pml4());

        Process {
            pid: 0,
            egid,
            euid,
            gid,
            ppid: 0,
            pgid: 0,
            uid,
            cwd,
            mapped_regions: Vec::new(),
            main_thread: create_user_thread(),
            pml4_phys: pml4,
            file_descriptors: Vec::with_capacity(8),
            file_descriptors_allocated: 0,
        }
    }

    // TODO: arch specific
    fn map_region(&self, region: &MappedRegion) {
        let vmm = virt::VIRTUAL_MEMORY_MANAGER.lock();

        let addr_base = region.start as u64;
        for i in 0..region.pages {
            let virt = VirtAddr::new(addr_base + i as u64 * PAGE_SIZE_4KIB);
            let phys = if region.flags.contains(MappedRegionFlags::ALLOC_ON_ACCESS) {
                PhysAddr::zero()
            } else {
                phys::alloc()
            };

            let flags = region.page_flags();
            vmm.map_4kib(self.pml4_phys, virt, phys, flags);
        }
    }

    // TODO: error
    pub fn add_region(
        &mut self,
        start_addr: usize,
        pages: usize,
        flags: MappedRegionFlags,
    ) -> Result<(), ()> {
        println!(
            "add region {:#x} {:#x} pages {:?}",
            start_addr, pages, flags
        );
        assert!(start_addr % 4096 == 0);

        let end = start_addr + pages * PAGE_SIZE_4KIB as usize;
        let region = self
            .mapped_regions
            .iter()
            .find(|region| region.start < end && start_addr < region.end);

        if region.is_some() {
            return Err(());
        }

        // TODO: check for overlapping regions
        let region = MappedRegion::new(start_addr, pages, flags);
        self.map_region(&region);
        self.mapped_regions.push(region);

        Ok(())
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
            Some(ref f) => {
                let val = Mutex::new(((**f).lock()).clone());
                Arc::new(val)
            }
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
    // TODO
    proc.pgid = pid;

    processes[pid - 1] = Some(Arc::new(Mutex::new(proc)));

    pid
}

unsafe fn write_strings_on_stack(stack: *mut u64, strs: &[&str]) -> *mut u64 {
    const POINTER_SIZE: usize = core::mem::size_of::<usize>();

    let mut string_stack = stack as *mut u8;
    assert!(string_stack as usize % core::mem::size_of::<usize>() == 0);
    for s in strs.iter().rev() {
        let aligned_size = s.len() + POINTER_SIZE - (s.len() % POINTER_SIZE);
        string_stack = string_stack.offset(-(aligned_size as isize));

        let stack_str = slice::from_raw_parts_mut(string_stack, s.len());
        stack_str.copy_from_slice(s.as_bytes());

        let leftover_size = aligned_size - s.len();
        if leftover_size > 0 {
            let leftover_ptr = string_stack.offset(s.len() as isize);
            let leftover_area = slice::from_raw_parts_mut(leftover_ptr, leftover_size);
            for byte in leftover_area {
                *byte = 0;
            }
        }
    }

    string_stack as *mut u64
}

unsafe fn write_string_table_on_stack(
    strs: &[&str],
    mut table_stack: *mut u64,
    mut str_stack: u64,
) -> *mut u64 {
    const POINTER_SIZE: usize = core::mem::size_of::<usize>();

    table_stack = table_stack.offset(-1);
    *table_stack = 0; // array terminating NULL

    for s in strs.iter().rev() {
        let aligned_size = s.len() + POINTER_SIZE - (s.len() % POINTER_SIZE);
        str_stack = str_stack - aligned_size as u64;

        table_stack = table_stack.offset(-1);
        *table_stack = str_stack;
    }

    table_stack
}

unsafe fn write_argv_envp(stack_bottom: u64, args: &[&str], envvars: &[&str]) -> (u64, u64) {
    let mut stack = stack_bottom as *mut u64;
    let envp_start = write_strings_on_stack(stack, envvars);
    let envp_end = stack_bottom;

    let argv_start = write_strings_on_stack(envp_start, args);
    let argv_end = envp_start as u64;

    stack = argv_start;
    let envp = write_string_table_on_stack(envvars, stack, envp_end);
    let argv = write_string_table_on_stack(args, envp, argv_end);

    (argv as u64, envp as u64)
}

pub fn load_process(proc: &mut Process, exec_path: &str, args: &[&str], envvars: &[&str]) -> bool {
    println!("load process {}", exec_path);
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
        println!("loading segment {} {}", ph.p_vaddr, ph.p_memsz);

        let mut flags = MappedRegionFlags::empty();
        /*if ph.p_flags & PF_W > 0 {
            flags |= MappedRegionFlags::READ_WRITE;
        }*/
        // FIXME: remove READ_WRITE flag after we are done copying the memory from the file
        flags |= MappedRegionFlags::READ_WRITE;

        if ph.p_flags & PF_X > 0 {
            flags |= MappedRegionFlags::EXECUTE;
        }

        let seg_page_start = VirtAddr::new(ph.p_vaddr - ph.p_vaddr % PAGE_SIZE_4KIB);
        let pages = utils::div_and_ceil(ph.p_memsz as usize, PAGE_SIZE_4KIB as usize);
        proc.add_region(seg_page_start.get() as usize, pages, flags)
            .unwrap();

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
    const STACK_SIZE_IN_PAGES: u64 = 16; // 64 KiB
    const STACK_SIZE: u64 = STACK_SIZE_IN_PAGES * PAGE_SIZE_4KIB;

    proc.add_region(
        STACK_BASE as usize,
        STACK_SIZE_IN_PAGES as usize,
        MappedRegionFlags::READ_WRITE,
    )
    .unwrap();

    let stack_bottom = STACK_BASE + STACK_SIZE;
    let (argv, envp) = unsafe { write_argv_envp(stack_bottom, args, envvars) };

    assert!(argv % 8 == 0);
    let stack_top = argv - argv % 16;

    // argc, 1st arg
    main_thread.user_regs.rdi = args.len() as u64;
    // argv, 2nd arg
    main_thread.user_regs.rsi = argv;
    // envp, 3rd arg
    main_thread.user_regs.rdx = envp;

    // TODO: validate
    main_thread.user_regs.rip = elf_file.ehdr.e_entry;
    main_thread.user_regs.rsp = stack_top;

    println!("RSP: {:#x}", argv);

    main_thread.process_id = proc.pid;

    true
}

pub fn load_base_process(exec_path: &str) {
    let cwd = Arc::new(Mutex::new(
        *fs::open("/root").expect("Failed to open /root"),
    ));
    let pid = add_process(Process::new(1, 1, 1, 1, cwd));
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

    let argv = [<&str>::clone(&exec_path)];
    let envp = [];

    if !load_process(&mut proc, exec_path, &argv[..], &envp[..]) {
        panic!("failed to load base process");
    }

    run_user_thread(main_thread_id);
}

pub fn get_process(pid: usize) -> Option<Arc<Mutex<Process>>> {
    let processes = PROCESSES.lock();
    processes[pid - 1].clone()
}
