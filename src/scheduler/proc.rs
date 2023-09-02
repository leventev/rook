use crate::{
    arch::x86_64::{
        disable_interrupts, enable_interrupts, get_current_pml4,
        paging::PageFlags,
        syscall::proc::{CloneArgs, CloneFlags},
    },
    fs::{self, FileDescriptor},
    mm::{
        phys,
        virt::{switch_pml4, PAGE_SIZE_4KIB, PML4},
        PhysAddr, VirtAddr,
    },
    posix::{Stat, AT_FCWD, FileOpenFlags},
    scheduler::{ThreadInner, SCHEDULER},
    utils::slot_allocator::SlotAllocator,
};
use alloc::{
    boxed::Box,
    slice,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use elf::{
    abi::{PF_X, PT_LOAD, PT_TLS},
    endian::LittleEndian,
    segment::ProgramHeader,
    ElfBytes,
};
use spin::Mutex;

use super::{thread::ThreadLocalStorage, Thread, ThreadID};

bitflags::bitflags! {
    pub struct MappedRegionFlags: u64 {
        const READ_WRITE = 1 << 0;
        const ALLOC_ON_ACCESS = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

#[derive(Debug, Clone)]
pub struct MappedRegion {
    start: usize,
    pages: usize,
    end: usize,
    flags: MappedRegionFlags,
}

const MAX_PROCESSES: usize = 32;

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

#[derive(Debug)]
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
    pml4: PML4,
    file_descriptors: SlotAllocator<Arc<Mutex<FileDescriptor>>>,
}

unsafe impl Send for Process {}

static PROCESSES: Mutex<SlotAllocator<Arc<Mutex<Process>>>> = Mutex::new(SlotAllocator::new(None));

impl Process {
    fn create_base_process(cwd: Arc<Mutex<FileDescriptor>>) -> Arc<Mutex<Process>> {
        let mut processes = PROCESSES.lock();
        assert!(processes.allocated_slots() == 0);

        let current_pml4 = get_current_pml4();
        let new_pml4 = phys::alloc();
        current_pml4.copy_pml4_higher_half_entries(new_pml4);

        let new_pml4 = PML4::from_phys(new_pml4);

        let proc = Process {
            pid: 1,
            egid: 1,
            euid: 1,
            gid: 1,
            ppid: 0,
            pgid: 1,
            uid: 1,
            cwd,
            mapped_regions: Vec::new(),
            main_thread: SCHEDULER.create_user_thread(1),
            pml4: new_pml4,
            file_descriptors: SlotAllocator::new(None),
        };

        let proc_arc = Arc::new(Mutex::new(proc));

        processes.allocate(Some(0), proc_arc.clone());

        proc_arc
    }

    // TODO: arch specific
    fn map_region(&self, region: &MappedRegion) {
        let addr_base = region.start as u64;
        let flags = region.page_flags();
        for i in 0..region.pages {
            let virt = VirtAddr::new(addr_base + i as u64 * PAGE_SIZE_4KIB);
            let phys = if region.flags.contains(MappedRegionFlags::ALLOC_ON_ACCESS) {
                PhysAddr::zero()
            } else {
                phys::alloc()
            };

            self.pml4.map_4kib(virt, phys, flags);
        }
    }

    fn clear_file_descriptors(&mut self) {
        self.file_descriptors.clear();
    }

    pub fn change_cwd(&mut self, cwd: Arc<Mutex<FileDescriptor>>) {
        // TODO: the old cwd gets dropped here right?
        self.cwd = cwd;
    }

    // TODO: better name
    pub fn get_region(&self, region_start: usize, region_end: usize) -> Option<usize> {
        // TODO: check if addresses are aligned?
        self.mapped_regions
            .iter()
            .position(|region| region.start < region_end && region_start < region.end)
    }

    // TODO: error
    pub fn add_region(
        &mut self,
        region_start: usize,
        pages: usize,
        flags: MappedRegionFlags,
    ) -> Result<(), ()> {
        debug!(
            "add region {:#x} {:#x} pages {:?}",
            region_start, pages, flags
        );
        assert!(region_start % 4096 == 0);

        let region_end = region_start + pages * PAGE_SIZE_4KIB as usize;

        if self.get_region(region_start, region_end).is_some() {
            return Err(());
        }

        // TODO: check for overlapping regions
        let region = MappedRegion::new(region_start, pages, flags);
        self.map_region(&region);
        self.mapped_regions.push(region);

        Ok(())
    }

    // TODO: docs, debug_assert desired_addr is aligned, other checks...
    pub fn mmap(
        &mut self,
        desired_addr: Option<usize>,
        len: usize,
        flags: MappedRegionFlags,
    ) -> Result<usize, ()> {
        // TODO: optimize
        let pages = len.div_ceil(4096);
        let region_start = desired_addr.unwrap_or_else(|| {
            const REGION_SEARCH_START: usize = 0x1000;
            let (mut start, mut end) = (REGION_SEARCH_START, REGION_SEARCH_START + len);

            while let Some(idx) = self.get_region(start, end) {
                let region = &self.mapped_regions[idx];
                start = region.end + 0x1000;
                end = start + len;
            }

            start
        });

        self.add_region(region_start, pages, flags)?;
        Ok(region_start)
    }

    pub fn new_fd(
        &mut self,
        hint: Option<usize>,
        file_descriptor: Arc<Mutex<FileDescriptor>>,
    ) -> Result<usize, ()> {
        match self.file_descriptors.allocate(hint, file_descriptor) {
            Some(fd) => Ok(fd),
            None => Err(()),
        }
    }

    // TODO: error
    pub fn dup_fd(&mut self, hint: Option<usize>, fd: usize) -> Result<usize, ()> {
        let file_desc = match self.file_descriptors.get(fd) {
            Some(f) => {
                let val = Mutex::new(((**f).lock()).clone());
                Arc::new(val)
            }
            None => return Err(()),
        };

        self.new_fd(hint, file_desc)
    }

    pub fn free_fd(&mut self, fd: usize) {
        self.file_descriptors.deallocate(fd)
    }

    pub fn get_fd(&self, fd: usize) -> Option<Arc<Mutex<FileDescriptor>>> {
        self.file_descriptors.get(fd).cloned()
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

    pub fn clone_proc(&self, clone_args: &CloneArgs) -> Arc<Mutex<Process>> {
        let mut processes = PROCESSES.lock();

        let tid = self.main_thread.upgrade().unwrap().lock().id;

        let clone_flags = CloneFlags::from_bits_truncate(clone_args.flags);

        let pml4 = if clone_flags.contains(CloneFlags::CLONE_VM) {
            self.pml4.clone()
        } else {
            let new_pml4 = phys::alloc();
            self.pml4.copy_page_tables(new_pml4);
            PML4::from_phys(new_pml4)
        };

        let proc = Process {
            pid: 0,
            ppid: self.pid,
            pgid: self.pgid,
            uid: self.uid,
            euid: self.euid,
            gid: self.gid,
            egid: self.egid,
            cwd: self.cwd.clone(),
            // TODO: mapped regions?
            mapped_regions: self.mapped_regions.clone(),
            main_thread: Weak::new(),
            pml4,
            file_descriptors: self.file_descriptors.clone(),
        };

        let proc_arc = Arc::new(Mutex::new(proc));
        let pid = processes.allocate(None, proc_arc.clone()).unwrap() + 1;

        // unfortunately we can't allocate a pid without setting the value and
        // adding that functionality to SlotAllocator would introduce unnecessary
        // complexity and while this solution isn't the cleanest it's the easiest
        // for now
        {
            let proc = Arc::clone(&proc_arc);
            let mut proc = proc.lock();

            proc.pid = pid;
            proc.main_thread = SCHEDULER.copy_user_thread(pid, tid);
        }

        proc_arc
    }

    pub fn execve(&mut self, exec_path: &str, args: &[&str], envvars: &[&str]) -> Result<(), ()> {
        self.clear_file_descriptors();
        self.load_from_file(exec_path, args, envvars)?;
        self.open_std_streams();

        Ok(())
    }

    fn load_tls_segment(
        &mut self,
        file: &[u8],
        header: &ProgramHeader,
    ) -> Result<ThreadLocalStorage, ()> {
        // TODO: thread structure
        const TLS_BASE: VirtAddr = VirtAddr::new(0xfffffd0000000000);
        self.load_segment(file, header, TLS_BASE)?;

        let tls = ThreadLocalStorage::new(TLS_BASE, header.p_memsz as usize);

        let thread_struct_addr = tls.thead_struct_addr().get();
        let ptr = thread_struct_addr as *mut u64;
        unsafe {
            *ptr = thread_struct_addr;
        }

        Ok(tls)
    }

    fn load_normal_segment(&mut self, file: &[u8], header: &ProgramHeader) -> Result<(), ()> {
        self.load_segment(file, header, VirtAddr::new(header.p_vaddr))
    }

    fn load_segment(
        &mut self,
        file: &[u8],
        header: &ProgramHeader,
        virt_addr_start: VirtAddr,
    ) -> Result<(), ()> {
        let mut flags = MappedRegionFlags::empty();
        /*if ph.p_flags & PF_W > 0 {
            flags |= MappedRegionFlags::READ_WRITE;
        }*/
        // FIXME: remove READ_WRITE flag after we are done copying the memory from the file
        flags |= MappedRegionFlags::READ_WRITE;

        if header.p_flags & PF_X > 0 {
            flags |= MappedRegionFlags::EXECUTE;
        }

        let mem_size = header.p_memsz as usize;

        let seg_page_start = VirtAddr::new(virt_addr_start.get() - virt_addr_start.page_offset());
        let pages = mem_size.div_ceil(PAGE_SIZE_4KIB as usize);
        self.add_region(seg_page_start.get() as usize, pages, flags)
            .unwrap();

        let seg_size = header.p_filesz as usize;
        if seg_size > 0 {
            let seg_start = header.p_offset as usize;
            let seg_end = seg_start + seg_size;

            let proc_mem =
                unsafe { slice::from_raw_parts_mut(virt_addr_start.get() as *mut u8, seg_size) };
            let seg_mem = &file[seg_start..seg_end];

            proc_mem.copy_from_slice(seg_mem);
        }

        let remaining = mem_size - seg_size;
        if remaining > 0 {
            let ptr = (virt_addr_start.get() + seg_size as u64) as *mut u8;
            let seg_mem = unsafe { slice::from_raw_parts_mut(ptr, remaining) };
            seg_mem.fill(0);
        }

        Ok(())
    }

    fn load_segments(
        &mut self,
        file: &[u8],
        elf_file: &ElfBytes<'_, LittleEndian>,
    ) -> Result<Option<ThreadLocalStorage>, ()> {
        let segments = match elf_file.segments() {
            Some(segs) => segs,
            None => return Err(()),
        };

        // TODO TODO
        // FIXME

        let mut tls: Option<ThreadLocalStorage> = None;

        // TODO: check if the segments are in userspace
        for ph in segments {
            match ph.p_type {
                PT_LOAD => self.load_normal_segment(file, &ph).unwrap(),
                PT_TLS => match tls {
                    Some(_) => warn!(
                        "ignoring PT_TLS segment because we have already loaded one, segment: {:?}",
                        ph
                    ),
                    None => (), //tls = self.load_tls_segment(file, &ph).ok(),
                },
                _ => {
                    warn!("ignoring segment: {:?}", ph);
                    continue;
                }
            };
        }

        Ok(tls)
    }

    pub fn load_from_file(
        &mut self,
        exec_path: &str,
        args: &[&str],
        envvars: &[&str],
    ) -> Result<(), ()> {
        // TODO: shorten this function
        let current_pml4 = get_current_pml4();
        let new_pml4 = phys::alloc();
        current_pml4.copy_pml4_higher_half_entries(new_pml4);
        self.pml4 = PML4::from_phys(new_pml4);
        // TODO: cleanup pml4 from fork

        self.mapped_regions.clear();

        // TODO: proper flags
        let mut fd = fs::open(exec_path, FileOpenFlags::empty()).unwrap();

        let mut stat_buf = Stat::zero();
        fd.stat(&mut stat_buf).unwrap();

        let file_size = stat_buf.st_size as usize;

        // TODO: perhaps we can parse the ELF header without reading the whole file
        // and instead later reading the file to userspace
        // TODO: don't unnecessarily zero the memory
        let mut buff: Box<[u8]> = vec![0; file_size].into_boxed_slice();
        match fd.read(file_size, &mut buff[..]) {
            Ok(_) => {}
            Err(err) => panic!("{:?}", err),
        };

        let elf_file = match ElfBytes::<LittleEndian>::minimal_parse(&buff[..]) {
            Ok(file) => file,
            Err(_) => return Err(()),
        };

        switch_pml4(&self.pml4);
        let tls = self.load_segments(&buff, &elf_file).unwrap();

        const STACK_BASE: u64 = 0xfffffd8000000000;
        const STACK_SIZE_IN_PAGES: u64 = 16; // 64 KiB
        const STACK_SIZE: u64 = STACK_SIZE_IN_PAGES * PAGE_SIZE_4KIB;

        self.add_region(
            STACK_BASE as usize,
            STACK_SIZE_IN_PAGES as usize,
            MappedRegionFlags::READ_WRITE,
        )
        .unwrap();

        let argc_argv_envp_size = (1 + args.len() + 1 + envvars.len() + 1) * 8;
        let rem = argc_argv_envp_size % 16;
        let stack_bottom = STACK_BASE + STACK_SIZE - rem as u64;

        let (argv, envp) = unsafe { write_argv_envp(stack_bottom, args, envvars) };

        let stack_top = argv - 8;
        {
            let stack_ptr = stack_top as *mut u64;
            unsafe {
                stack_ptr.write(args.len() as u64);
            }
        }

        debug!(
            "stack_top: {:#x} argc: {:#x} argv: {:#x} envp: {:#x}",
            stack_top,
            args.len(),
            argv,
            envp
        );

        // FIXME: random deadlock caused by timer interrupt
        // maybe disable interrupts here

        let main_thread_lock = self.main_thread.upgrade().unwrap();
        let mut main_thread = main_thread_lock.lock();

        if let ThreadInner::User(data) = &mut main_thread.inner {
            // argc, 1st arg
            data.user_regs.general.rdi = args.len() as u64;
            // argv, 2nd arg
            data.user_regs.general.rsi = argv;
            // envp, 3rd arg
            data.user_regs.general.rdx = envp;

            // TODO: validate
            data.user_regs.rip = elf_file.ehdr.e_entry;
            data.user_regs.rsp = stack_top;

            data.tls = tls;

            data.pid = self.pid;
        } else {
            unreachable!()
        }

        Ok(())
    }

    fn open_std_streams(&mut self) {
        // open console
        // TODO: proper flags
        let console_fd = fs::open("/dev/console", FileOpenFlags::O_RDWR).expect("Failed to open /dev/console");

        // stdin
        let fd = self
            .new_fd(Some(0), Arc::new(Mutex::new(*console_fd)))
            .unwrap();
        assert!(fd == 0);

        // stdout
        let fd = self.dup_fd(None, fd).unwrap();
        assert!(fd == 1);

        // stderr
        let fd = self.dup_fd(None, fd).unwrap();
        assert!(fd == 2);
    }
}

unsafe fn write_strings_on_stack(stack: *mut u64, strs: &[&str]) -> *mut u64 {
    const POINTER_SIZE: usize = core::mem::size_of::<usize>();

    let mut string_stack = stack as *mut u8;
    assert!(string_stack as usize % POINTER_SIZE == 0);
    for s in strs.iter().rev() {
        let aligned_size = s.len() + POINTER_SIZE - (s.len() % POINTER_SIZE);
        string_stack = string_stack.offset(-(aligned_size as isize));

        let stack_str = slice::from_raw_parts_mut(string_stack, s.len());
        stack_str.copy_from_slice(s.as_bytes());

        let leftover_size = aligned_size - s.len();
        if leftover_size > 0 {
            let leftover_ptr = string_stack.add(s.len());
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
        str_stack -= aligned_size as u64;

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

pub fn load_base_process(exec_path: &str) {
    disable_interrupts();

    let main_thread_id: ThreadID;

    {
        let cwd = Arc::new(Mutex::new(
            // TODO: proper flags
            *fs::open("/root", FileOpenFlags::empty()).expect("Failed to open /root"),
        ));

        let proc_lock = Process::create_base_process(cwd);
        let mut proc = proc_lock.lock();

        proc.open_std_streams();

        main_thread_id = proc.main_thread.upgrade().unwrap().lock().id;

        let argv = [<&str>::clone(&exec_path)];
        let envp = ["HOME=/root"];

        proc.load_from_file(exec_path, &argv[..], &envp[..])
            .expect("Failed to load base process");
    }

    SCHEDULER.run_thread(main_thread_id);
    enable_interrupts();
}

pub fn get_process(pid: usize) -> Option<Arc<Mutex<Process>>> {
    let processes = PROCESSES.lock();
    let proc = processes.get(pid - 1);
    proc.map(Arc::clone)
}
