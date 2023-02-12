bits 64

section .text
global x86_64_get_rsp:function (x86_64_get_rsp.end - x86_64_get_rsp)
x86_64_get_rsp:
    mov rax, rsp
    ret
.end:

global x86_64_get_rflags:function (x86_64_get_rflags.end - x86_64_get_rflags)
x86_64_get_rflags:
    pushfq
    pop rax
    ret
.end:

global x86_64_set_cr3:function (x86_64_set_cr3.end - x86_64_set_cr3)
x86_64_set_cr3:
    mov cr3, rdi
    ret
.end:

global x86_64_get_cr3:function (x86_64_get_cr3.end - x86_64_get_cr3)
x86_64_get_cr3:
    mov rax, cr3
    ret
.end:

global x86_64_switch_task:function (x86_64_switch_task.end - x86_64_switch_task)
x86_64_switch_task:
    add rsp, 8 ; we dont need the return value
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

    ; set segments
    push rax

    mov rax, [rsp + 8]
    mov es, rax

    mov rax, [rsp + 16]
    mov ds, rax

    mov rax, [rsp + 24]
    mov fs, rax

    mov rax, [rsp + 32]
    mov gs, rax

    pop rax

    ; add es, ds, fs, gs
    add rsp, 32

    ; the iret parameters are already pushed to the stack
    iretq
.end:

extern __block_current_thread
extern save_regs
global x86_64_block_task:function (x86_64_block_task.end - x86_64_block_task)
x86_64_block_task:
    push rbp
    mov rbp, rsp

    ; push ss
    mov rax, ss
    push rax

    ; push rsp
    mov rax, rsp
    push rax

    ; push rflags
    pushfq

    ; push cs
    mov rax, cs
    push rax

    ; push rip
    mov rax, .return
    push rax

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
    push rax

    ; save registers
    call save_regs
    call  __block_current_thread

.return:
    pop rbp
    ret
.end:
