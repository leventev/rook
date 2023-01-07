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

#[no_mangle]
pub fn excp_div_by_zero() -> ! {
    panic!("excp_div_by_zero");
}

#[no_mangle]
pub fn excp_debug() -> ! {
    panic!("excp_debug");
}

#[no_mangle]
pub fn excp_non_maskable_interrutpt() -> ! {
    panic!("excp_non_maskable_interrutpt");
}

#[no_mangle]
pub fn excp_breakpoint() -> ! {
    panic!("excp_breakpoint");
}

#[no_mangle]
pub fn excp_overflow() -> ! {
    panic!("excp_overflow");
}

#[no_mangle]
pub fn excp_bound_range_exceeded() -> ! {
    panic!("excp_bound_range_exceeded");
}

#[no_mangle]
pub fn excp_invalid_opcode() -> ! {
    panic!("excp_invalid_opcode");
}

#[no_mangle]
pub fn excp_device_not_available() -> ! {
    panic!("excp_device_not_available");
}

#[no_mangle]
pub fn excp_double_fault() -> ! {
    panic!("excp_double_fault");
}

#[no_mangle]
pub fn excp_coprocessor_segment_overrun() -> ! {
    panic!("excp_coprocessor_segment_overrun");
}

#[no_mangle]
pub fn excp_invalid_tss() -> ! {
    panic!("excp_invalid_tss");
}

#[no_mangle]
pub fn excp_segment_not_present() -> ! {
    panic!("excp_segment_not_present");
}

#[no_mangle]
pub fn excp_stack_segment_fault() -> ! {
    panic!("excp_stack_segment_fault");
}

#[no_mangle]
pub fn excp_general_protection_fault() -> ! {
    panic!("excp_general_protection_fault");
}

#[no_mangle]
pub fn excp_page_fault() -> ! {
    panic!("excp_page_fault");
}

#[no_mangle]
pub fn excp_x87() -> ! {
    panic!("excp_x87");
}

#[no_mangle]
pub fn excp_alignment_check() -> ! {
    panic!("excp_alignment_check");
}

#[no_mangle]
pub fn excp_machine_check() -> ! {
    panic!("excp_machine_check");
}

#[no_mangle]
pub fn excp_simd_fpe() -> ! {
    panic!("excp_simd_fpe");
}

#[no_mangle]
pub fn excp_virtualization() -> ! {
    panic!("excp_virtualization");
}

#[no_mangle]
pub fn excp_control_protection() -> ! {
    panic!("excp_control_protection");
}

#[no_mangle]
pub fn excp_hypervisor_injection() -> ! {
    panic!("excp_hypervisor_injection");
}

#[no_mangle]
pub fn excp_vmm_communication() -> ! {
    panic!("excp_vmm_communication");
}

#[no_mangle]
pub fn excp_security() -> ! {
    panic!("excp_security");
}
