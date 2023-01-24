bits 64

extern ata_interrupt

section .text
global __ata_interrupt:function (__ata_interrupt.end - __ata_interrupt)
__ata_interrupt:
    cli
    call ata_interrupt
.end: