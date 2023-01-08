use super::{inb, outb, idt};

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const ICW1_ICW4: u8 = 0x01; /* ICW4 (not) needed */
const ICW1_SINGLE: u8 = 0x02; /* Single (cascade) mode */
const ICW1_INTERVAL: u8 = 0x04; /* Call address interval 4 (8) */
const ICW1_LEVEL: u8 = 0x08; /* Level triggered (edge) mode */
const ICW1_INIT: u8 = 0x10; /* Initialization - required! */

const ICW4_8086: u8 = 0x01;
const ICW4_AUTO: u8 = 0x02;
const ICW4_BUF_SLAVE: u8 = 0x08;
const ICW4_BUF_MASTER: u8 = 0x0C;
const ICW4_SFNM: u8 = 0x10;

const PIC_EOI: u8 = 0x20;

const IDT_IRQ_BASE: usize = 32;

fn io_wait() {
    outb(0x80, 0);
}

fn outb_with_wait(port: u16, data: u8) {
    outb(port, data);
    io_wait();
}

fn inb_with_wait(port: u16) -> u8 {
    let val = inb(port);
    io_wait();
    val
}

pub fn init() {
    // save masks
    let master_mask = inb_with_wait(PIC1_DATA);
    let slave_mask = inb_with_wait(PIC1_DATA);

    // init
    outb_with_wait(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    outb_with_wait(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);

    // vector offset
    outb_with_wait(PIC1_DATA, 0x20);
    outb_with_wait(PIC1_DATA, 0x28);

    // set cascade port
    outb_with_wait(PIC1_DATA, 4);
    outb_with_wait(PIC2_DATA, 2);

    // set mode
    outb_with_wait(PIC1_DATA, ICW4_8086);
    outb_with_wait(PIC2_DATA, ICW4_8086);

    // reload masks
    outb_with_wait(PIC1_DATA, master_mask);
    outb_with_wait(PIC2_DATA, slave_mask);

    for i in 0..15 {
        set_irq(i);
    }
}

pub fn set_irq(irq: u8) {
    let mut irq_num = irq;
    let port = if irq >= 8 {
        irq_num -= 8;
        PIC2_DATA
    } else {
        PIC1_DATA
    };

    let mask = inb(port) | (1 << irq_num);
    outb(port, mask);
}

pub fn clear_irq(irq: u8) {
    let mut irq_num = irq;
    let port = if irq >= 8 {
        irq_num -= 8;
        PIC2_DATA
    } else {
        PIC1_DATA
    };

    let mask = inb(port) & !(1 << irq_num);
    outb(port, mask);
}

pub fn send_irq_eoi(irq: u8) {
    let port = if irq >= 8 {
        PIC2_COMMAND
    } else {
        PIC1_COMMAND
    };
    outb(port, PIC_EOI);
}

pub fn install_irq_handler(irq: u8, handler: u64) {
    assert!(irq < 16);
    idt::install_interrupt_handler(IDT_IRQ_BASE + irq as usize, handler);
}
