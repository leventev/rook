bits 64

extern pit_timer_interrupt
extern ticks_until_switch
extern save_regs

section .data
rax_temp: dq 0

; this is wildly unoptimized and slow
section .text
global __pit_timer_interrupt:function (__pit_timer_interrupt.end - __pit_timer_interrupt)
__pit_timer_interrupt:
    mov [rax_temp], rax

    mov rax, gs
    push rax

    mov rax, fs
    push rax

    mov rax, ds
    push rax

    mov rax, es
    push rax

    push rbp
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx

    mov rax, [rax_temp]
    push rax

    ; save registers
    call save_regs
.call_timer:
    call pit_timer_interrupt

    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    pop rbp

    ; segments
    add rsp, 4 * 8

    iretq
.end: