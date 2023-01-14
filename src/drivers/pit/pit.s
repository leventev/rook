bits 64

extern pit_timer_interrupt
extern ticks_until_switch
extern save_regs

; this is wildly unoptimized and slow
section .text
global __pit_timer_interrupt:function (__pit_timer_interrupt.end - __pit_timer_interrupt)
__pit_timer_interrupt:
    cli
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    push rbp

    ; check if we have to save the registers
    call ticks_until_switch
    cmp rax, 0
    jne .call_timer

    mov rax, es
    push rax

    mov rax, ds
    push rax

    mov rax, fs
    push rax

    mov rax, gs
    push rax

    mov rbx, [esp + 20 * 8]

    ; push rip
    mov rax, [rbx]
    push rax

    ; push cs
    mov rax, [rbx + 8]
    push rax

    ; push rflags
    mov rax, [rbx + 16]
    push rax

    ; push rsp
    mov rax, [rbx + 24]
    push rax

    ; push ss
    mov rax, [rbx + 32]
    push rax

    ; save registers
    call save_regs

    ; reset stack
    add esp, 9 * 8
.call_timer:
    call pit_timer_interrupt

    pop rbp
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    iretq
.end: