use crate::{
    arch::x86_64::{get_cr2, get_current_pml4, paging::PageFlags},
    mm::{
        phys,
        virt::{PAGE_SIZE_4KIB, VIRTUAL_MEMORY_MANAGER},
        VirtAddr,
    },
    scheduler::RegisterState,
};

extern "C" {
    pub fn __excp_div_by_zero();
    pub fn __excp_debug();
    pub fn __excp_non_maskable_interrutpt();
    pub fn __excp_breakpoint();
    pub fn __excp_overflow();
    pub fn __excp_bound_range_exceeded();
    pub fn __excp_invalid_opcode();
    pub fn __excp_device_not_available();
    pub fn __excp_double_fault();
    pub fn __excp_coprocessor_segment_overrun();
    pub fn __excp_invalid_tss();
    pub fn __excp_segment_not_present();
    pub fn __excp_stack_segment_fault();
    pub fn __excp_general_protection_fault();
    pub fn __excp_page_fault();
    pub fn __excp_x87();
    pub fn __excp_alignment_check();
    pub fn __excp_machine_check();
    pub fn __excp_simd_fpe();
    pub fn __excp_virtualization();
    pub fn __excp_control_protection();
    pub fn __excp_hypervisor_injection();
    pub fn __excp_vmm_communication();
    pub fn __excp_security();
}

bitflags::bitflags! {
    struct PageFaultFlags: u32 {
        const PRESENT = 1 << 0;
        const WRITE = 1 << 1;
        const USER = 1 << 2;
        const RESERVED_WRITE = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 5;
        const PROTECTION_KEY = 1 << 6;
        const SHADOW_STACK = 1 << 7;
    }
}

#[no_mangle]
pub static mut EXCEPTION_REG_STATE: RegisterState = RegisterState::zero();

#[no_mangle]
pub extern "C" fn excp_div_by_zero() -> ! {
    panic!("excp_div_by_zero");
}

#[no_mangle]
pub extern "C" fn excp_debug() -> ! {
    panic!("excp_debug");
}

#[no_mangle]
pub extern "C" fn excp_non_maskable_interrutpt() -> ! {
    panic!("excp_non_maskable_interrutpt");
}

#[no_mangle]
pub extern "C" fn excp_breakpoint() -> ! {
    panic!("excp_breakpoint");
}

#[no_mangle]
pub extern "C" fn excp_overflow() -> ! {
    panic!("excp_overflow");
}

#[no_mangle]
pub extern "C" fn excp_bound_range_exceeded() -> ! {
    panic!("excp_bound_range_exceeded");
}

#[no_mangle]
pub extern "C" fn excp_invalid_opcode() -> ! {
    panic!("excp_invalid_opcode");
}

#[no_mangle]
pub extern "C" fn excp_device_not_available() -> ! {
    panic!("excp_device_not_available");
}

#[no_mangle]
pub extern "C" fn excp_double_fault() -> ! {
    panic!("excp_double_fault");
}

#[no_mangle]
pub extern "C" fn excp_coprocessor_segment_overrun() -> ! {
    panic!("excp_coprocessor_segment_overrun");
}

#[no_mangle]
pub extern "C" fn excp_invalid_tss() -> ! {
    panic!("excp_invalid_tss");
}

#[no_mangle]
pub extern "C" fn excp_segment_not_present() -> ! {
    panic!("excp_segment_not_present");
}

#[no_mangle]
pub extern "C" fn excp_stack_segment_fault() -> ! {
    panic!("excp_stack_segment_fault");
}

#[no_mangle]
pub extern "C" fn excp_general_protection_fault(error_code: u64) -> ! {
    println!("ERROR: {:#x}", error_code);
    println!("{}", unsafe { EXCEPTION_REG_STATE });
    panic!("GENERAL PROTECTION FAULT");
}

#[no_mangle]
pub extern "C" fn excp_page_fault(error_code: u64) {
    assert!(!VIRTUAL_MEMORY_MANAGER.is_locked());
    let vmm = VIRTUAL_MEMORY_MANAGER.lock();
    let pml4 = get_current_pml4();

    let page_fault_flags = PageFaultFlags::from_bits(error_code as u32).unwrap();

    let addr = VirtAddr::new(unsafe { get_cr2() });
    let (_, mut page_flags) = vmm.get_page_entry_from_virt(pml4, addr).unwrap();

    println!("{:?} ... {:?} {}", page_fault_flags, page_flags, addr);

    if page_flags.contains(PageFlags::ALLOC_ON_ACCESS) {
        let page_virt = addr - VirtAddr::new(addr.get() % PAGE_SIZE_4KIB);
        let page_phys = phys::alloc();
        page_flags.remove(PageFlags::ALLOC_ON_ACCESS);
        page_flags.insert(PageFlags::PRESENT);
        vmm.map_4kib(pml4, page_virt, page_phys, page_flags);
        println!("alloc on access");
        return;
    }

    let page_present = page_fault_flags.contains(PageFaultFlags::PRESENT);
    assert_eq!(page_present, page_flags.contains(PageFlags::PRESENT));

    let write_read_only_page = page_fault_flags.contains(PageFaultFlags::WRITE)
        && !page_flags.contains(PageFlags::READ_WRITE);

    println!("ERROR FLAGS: {:?}", page_fault_flags);
    println!("PAGE FLAGS: {:?}", page_flags);
    println!("{}", unsafe { EXCEPTION_REG_STATE });
    if !page_present {
        println!("tried to access a non present page");
    } else if write_read_only_page {
        println!("tried to write to a read-only page");
    } else {
        unreachable!()
    }
    panic!("PAGE FAULT");
    // TODO: SIGSEGV
}

#[no_mangle]
pub extern "C" fn excp_x87() -> ! {
    panic!("excp_x87");
}

#[no_mangle]
pub extern "C" fn excp_alignment_check() -> ! {
    panic!("excp_alignment_check");
}

#[no_mangle]
pub extern "C" fn excp_machine_check() -> ! {
    panic!("excp_machine_check");
}

#[no_mangle]
pub extern "C" fn excp_simd_fpe() -> ! {
    panic!("excp_simd_fpe");
}

#[no_mangle]
pub extern "C" fn excp_virtualization() -> ! {
    panic!("excp_virtualization");
}

#[no_mangle]
pub extern "C" fn excp_control_protection() -> ! {
    panic!("excp_control_protection");
}

#[no_mangle]
pub extern "C" fn excp_hypervisor_injection() -> ! {
    panic!("excp_hypervisor_injection");
}

#[no_mangle]
pub extern "C" fn excp_vmm_communication() -> ! {
    panic!("excp_vmm_communication");
}

#[no_mangle]
pub extern "C" fn excp_security() -> ! {
    panic!("excp_security");
}
