bits 64

%macro save_gprs 0
    ; gpr
    mov [EXCEPTION_REG_STATE +  0 * 8], rax
    mov [EXCEPTION_REG_STATE +  1 * 8], rbx
    mov [EXCEPTION_REG_STATE +  2 * 8], rcx
    mov [EXCEPTION_REG_STATE +  3 * 8], rdx
    mov [EXCEPTION_REG_STATE +  4 * 8], rsi
    mov [EXCEPTION_REG_STATE +  5 * 8], rdi
    mov [EXCEPTION_REG_STATE +  6 * 8], r8
    mov [EXCEPTION_REG_STATE +  7 * 8], r9
    mov [EXCEPTION_REG_STATE +  8 * 8], r10
    mov [EXCEPTION_REG_STATE +  9 * 8], r11
    mov [EXCEPTION_REG_STATE + 10 * 8], r12
    mov [EXCEPTION_REG_STATE + 11 * 8], r13
    mov [EXCEPTION_REG_STATE + 12 * 8], r14
    mov [EXCEPTION_REG_STATE + 13 * 8], r15
    mov [EXCEPTION_REG_STATE + 14 * 8], rbp

    ; segment regs
    mov rax, es
    mov [EXCEPTION_REG_STATE + 15 * 8], rax
    mov rax, ds
    mov [EXCEPTION_REG_STATE + 16 * 8], rax
    mov rax, fs
    mov [EXCEPTION_REG_STATE + 17 * 8], rax
    mov rax, gs
    mov [EXCEPTION_REG_STATE + 18 * 8], rax
%endmacro

%macro restore_gprs 0
    mov rax, [EXCEPTION_REG_STATE +  0 * 8]
    mov rbx, [EXCEPTION_REG_STATE +  1 * 8]
    mov rcx, [EXCEPTION_REG_STATE +  2 * 8]
    mov rdx, [EXCEPTION_REG_STATE +  3 * 8]
    mov rsi, [EXCEPTION_REG_STATE +  4 * 8]
    mov rdi, [EXCEPTION_REG_STATE +  5 * 8]
    mov r8,  [EXCEPTION_REG_STATE +  6 * 8]
    mov r9,  [EXCEPTION_REG_STATE +  7 * 8]
    mov r10, [EXCEPTION_REG_STATE +  8 * 8]
    mov r11, [EXCEPTION_REG_STATE +  9 * 8]
    mov r12, [EXCEPTION_REG_STATE + 10 * 8]
    mov r13, [EXCEPTION_REG_STATE + 11 * 8]
    mov r14, [EXCEPTION_REG_STATE + 12 * 8]
    mov r15, [EXCEPTION_REG_STATE + 13 * 8]
    mov rbp, [EXCEPTION_REG_STATE + 14 * 8]
%endmacro


%macro save_iret_data 1
    push rax

    ; rip
    mov rax, [rsp + (%1 + 1) * 8]
    mov [EXCEPTION_REG_STATE + 0xB0], rax

    ; cs
    mov rax, [rsp + (%1 + 2) * 8]
    mov [EXCEPTION_REG_STATE + 0xA0], rax

    ; rflags
    mov rax, [rsp + (%1 + 3) * 8]
    mov [EXCEPTION_REG_STATE + 0xA8], rax

    ; rsp
    mov rax, [rsp + (%1 + 4) * 8]
    mov [EXCEPTION_REG_STATE + 0xB8], rax

    ; ss
    mov rax, [rsp + (%1 + 5) * 8]
    mov [EXCEPTION_REG_STATE + 0x98], rax

    pop rax

%endmacro

%macro exception_handler 1
extern excp_ %+ %1
global __excp_ %+ %1:function (%%end - __excp_ %+ %1)
__excp_ %+ %1:
    cli

    save_iret_data 0
    save_gprs

    call excp_ %+ %1
    iretq
%%end:
%endmacro

extern EXCEPTION_REG_STATE

%macro exception_handler_error_code 1
extern excp_ %+ %1
global __excp_ %+ %1:function (%%end - __excp_ %+ %1)
__excp_ %+ %1:
    cli

    save_iret_data 1
    save_gprs

    ; error code
    pop rdi
    call excp_ %+ %1

    iretq ; unreachable
%%end:
%endmacro

%macro exception_handler_error_code_return 1
extern excp_ %+ %1
global __excp_ %+ %1:function (%%end - __excp_ %+ %1)
__excp_ %+ %1:
    cli

    save_iret_data 1
    save_gprs

    ; error code
    pop rdi
    call excp_ %+ %1
    restore_gprs

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
exception_handler_error_code double_fault
exception_handler coprocessor_segment_overrun
exception_handler_error_code invalid_tss
exception_handler_error_code segment_not_present
exception_handler_error_code stack_segment_fault
exception_handler_error_code general_protection_fault
exception_handler_error_code_return page_fault
exception_handler x87
exception_handler_error_code alignment_check
exception_handler machine_check
exception_handler simd_fpe
exception_handler virtualization
exception_handler_error_code control_protection
exception_handler hypervisor_injection
exception_handler_error_code vmm_communication
exception_handler_error_code security
