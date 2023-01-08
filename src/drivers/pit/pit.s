bits 64

extern pit_timer_interrupt

section .text
global __pit_timer_interrupt:function (__pit_timer_interrupt.end - __pit_timer_interrupt)
__pit_timer_interrupt:
    cli
    call pit_timer_interrupt
    iretq
.end: