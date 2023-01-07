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
    mov rax, [rsp + 4]
    mov cr3, rax
    ret
.end:

global x86_64_get_cr3:function (x86_64_get_cr3.end - x86_64_get_cr3)
x86_64_get_cr3:
    mov rax, cr3
    ret
.end:
