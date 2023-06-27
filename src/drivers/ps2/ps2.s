bits 64

extern handle_key_event

section .data
rax_temp: dq 0

section .text
global __ps2_first_interrupt:function (__ps2_first_interrupt.end - __ps2_first_interrupt)
__ps2_first_interrupt:
    mov [rax_temp], rax

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

    call handle_key_event

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