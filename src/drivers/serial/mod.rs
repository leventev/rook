use crate::arch::x86_64::{inb, outb};

const COM1: u16 = 0x3F8;
const COM2: u16 = 0x2F8;
const COM3: u16 = 0x3E8;
const COM4: u16 = 0x2E8;
const COM5: u16 = 0x5F8;
const COM6: u16 = 0x4F8;
const COM7: u16 = 0x5E8;
const COM8: u16 = 0x4E8;

const DATA_REG: u16 = 0x0;
const INTERRUPT_ENABLE_REG: u16 = 0x1;
const FIFO_CONTROL_REG: u16 = 0x2;
const LINE_CONTROL_REG: u16 = 0x3;
const MODEM_CONTROL_REG: u16 = 0x4;
const LINE_STATUS_REG: u16 = 0x5;

// TODO: implement the whole driver

pub fn init() -> bool {
    // enable reg
    outb(COM1 + INTERRUPT_ENABLE_REG, 0);

    // set dlab
    outb(COM1 + LINE_CONTROL_REG, 0x80);

    // set baud rate to 3
    outb(COM1 + DATA_REG, 0x3);
    outb(COM1 + INTERRUPT_ENABLE_REG, 0x0);

    // disable dlab, 8 bits, no parity, one stop bit
    outb(COM1 + LINE_CONTROL_REG, 0x03);

    // enable fifo
    outb(COM1 + FIFO_CONTROL_REG, 0xC7);

    // IRQs enabled, RTS/DSR set
    outb(COM1 + MODEM_CONTROL_REG, 0x0B);

    // test if the chip exists
    outb(COM1 + MODEM_CONTROL_REG, 0x1E);
    outb(COM1 + DATA_REG, 0xAE);

    if inb(COM1 + DATA_REG) != 0xAE {
        return false;
    }

    // set to normal mode
    outb(COM1 + MODEM_CONTROL_REG, 0x0F);

    true
}

fn is_transmit_empty() -> bool {
    inb(COM1 + LINE_STATUS_REG) & 0x20 > 0
}

pub fn write(data: u8) {
    while !is_transmit_empty() {}
    outb(COM1 + DATA_REG, data);
}
