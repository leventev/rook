bits 64

extern vmm_setup
extern kernel_init

section .data
    hddm_adjust_offset: dq 0
    stack_bottom: dq 0
    gdtr_limit: dw 0
    gdtr_addr: dq 0

global hddm_adjust_offset

section .text
global _start:function (_start.end - _start)
_start:
    mov [stack_bottom], rsp

    call vmm_setup

    mov rax, [stack_bottom]
    add rax, [hddm_adjust_offset]
    mov rsp, rax

    sgdt [gdtr_limit]
    mov rax, [hddm_adjust_offset]
    add [gdtr_addr], rax
    lgdt [gdtr_limit]

    call kernel_init
.loop:
    jmp .loop
.end: