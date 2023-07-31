bits 64

extern pit_timer_interrupt

section .text
global __pit_timer_interrupt:function (__pit_timer_interrupt.end - __pit_timer_interrupt)
__pit_timer_interrupt:
    ; push general purpose registers
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
    push rax

    mov rdi, rsp
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

    iretq
.end: