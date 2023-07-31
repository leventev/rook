bits 64

extern GDT_DESCRIPTOR
extern handle_syscall
extern __block_current_thread

section .data
temp: dq 0
thread_regs_ptr: dq 0

section .text
global load_gdt:function (load_gdt.end - load_gdt)
load_gdt:
    lgdt [GDT_DESCRIPTOR]
    ; 0x08 is the kernel code segment
    push 0x08
    lea rax, [rel .reload_segments]
    push rax
    retfq

.reload_segments:
    ; 0x08 is the kernel data segment
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

.load_tr:
    ; 0x28 is the TSS low segment
    mov ax, 0x28 | 3
    ltr ax

    ret
.end:

global x86_64_switch_task:function (x86_64_switch_task.end - x86_64_switch_task)
x86_64_switch_task:
    ; rdi = *RegisterState

    ; save rdi
    mov rax, [rdi + 0x28]
    mov [temp], rax

    ; load general purpose registers
    mov rax, [rdi + 0x00]
    mov rbx, [rdi + 0x08]
    mov rcx, [rdi + 0x10]
    mov rdx, [rdi + 0x18]
    mov rsi, [rdi + 0x20]
    ; we will set rdi later
    mov r8,  [rdi + 0x30]
    mov r9,  [rdi + 0x38]
    mov r10, [rdi + 0x40]
    mov r11, [rdi + 0x48]
    mov r12, [rdi + 0x50]
    mov r13, [rdi + 0x58]
    mov r14, [rdi + 0x60]
    mov r15, [rdi + 0x68]
    mov rbp, [rdi + 0x70]

    ; save rax before we set segments and push iret params
    push rax

    ; set segments
    mov rax, [rdi + 0x78]
    mov es, rax
    mov rax, [rdi + 0x80]
    mov ds, rax
    mov rax, [rdi + 0x88]
    mov fs, rax
    mov rax, [rdi + 0x90]
    mov gs, rax

    ; push iret params
    ; ss
    mov rax, [rdi + 0x98]
    push rax
    ; rsp
    mov rax, [rdi + 0xB8]
    push rax
    ; rflags
    mov rax, [rdi + 0xA8]
    push rax
    ; cs
    mov rax, [rdi + 0xA0]
    push rax
    ; rip
    mov rax, [rdi + 0xB0]
    push rax

    ; load rax
    mov rax, [rsp + 5 * 8]

    ; load rdi
    mov rdi, [temp]

    iretq
.end:

global __handle_syscall:function (__handle_syscall.end - __handle_syscall)
__handle_syscall:
    ; set segments
    mov [temp], rax
    mov ax, 0x10
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov rax, [temp]

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
    call handle_syscall

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

global x86_64_block_task:function (x86_64_block_task.end - x86_64_block_task)
x86_64_block_task:
    hlt
    ;mov [temp], rax
;
    ;mov rax, [CURRENT_THREAD_REGS]
    ;mov [thread_regs_ptr], rax
;
    ;mov [rax + 0x08], rbx
    ;mov [rax + 0x10], rcx
    ;mov [rax + 0x18], rdx
    ;mov [rax + 0x20], rsi
    ;mov [rax + 0x28], rdi
    ;mov [rax + 0x30], r8
    ;mov [rax + 0x38], r9
    ;mov [rax + 0x40], r10
    ;mov [rax + 0x48], r11
    ;mov [rax + 0x50], r12
    ;mov [rax + 0x58], r13
    ;mov [rax + 0x60], r14
    ;mov [rax + 0x68], r15
    ;mov [rax + 0x70], rbp
;
    ;mov rbx, .return
    ;mov [rax + 0x98], rbx
;
    ;mov rbx, cs
    ;mov [rax + 0xA0], rbx
;
    ;pushfq
    ;pop rbx
    ;mov [rax + 0xA8], rbx
;
    ;mov [rax + 0xB0], rsp
;
    ;mov rbx, ss
    ;mov [rax + 0xB8], rbx
;
    ;mov rbx, [temp]
    ;mov [rax + 0x0], rbx
    ;mov rbx, [rax + 0x08]

    call  __block_current_thread

.return:
    ret
.end:
