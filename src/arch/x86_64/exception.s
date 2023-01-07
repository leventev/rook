bits 64

%macro exception_handler 1
extern excp_ %+ %1
global __excp_ %+ %1:function (%%end - __excp_ %+ %1)
__excp_ %+ %1:
    call excp_ %+ %1
    iretq
%%end:
%endmacro

exception_handler div_by_zero
exception_handler debug
exception_handler non_maskable_interrutpt
exception_handler breakpoint
exception_handler overflow
exception_handler bound_range_exceeded
exception_handler invalid_opcode
exception_handler device_not_available
exception_handler double_fault
exception_handler coprocessor_segment_overrun
exception_handler invalid_tss
exception_handler segment_not_present
exception_handler stack_segment_fault
exception_handler general_protection_fault
exception_handler page_fault
exception_handler x87
exception_handler alignment_check
exception_handler machine_check
exception_handler simd_fpe
exception_handler virtualization
exception_handler control_protection
exception_handler hypervisor_injection
exception_handler vmm_communication
exception_handler security







