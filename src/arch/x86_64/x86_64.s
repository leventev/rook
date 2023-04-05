bits 64

extern GDT_DESCRIPTOR

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
    add rsp, 8 ; we dont need the return address
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


extern handle_syscall
global __handle_syscall:function (__handle_syscall.end - __handle_syscall)
__handle_syscall:
    ; set SS 
    shl rax, 16
    mov ax, 0x10
    mov ss, ax
    shr rax, 16

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

    ; AMD64 ABI function call 4th argument is RCX but 4th argument in a syscall is r10
    mov rcx, r10

    push rax
    call handle_syscall
    add rsp, 8

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
