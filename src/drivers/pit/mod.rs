use crate::arch::x86_64::registers::InterruptRegisters;
use crate::arch::x86_64::{
    outb,
    pic::{self, clear_irq, send_irq_eoi, set_irq},
};
use crate::scheduler::SCHEDULER;
use crate::time;

const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_CHANNEL1_DATA: u16 = 0x41;
const PIT_CHANNEL2_DATA: u16 = 0x42;
const PIT_MODE_CMD_REG: u16 = 0x43;

const PIT_SEL_CHANNEL0: u8 = 0b00 << 6;
const PIT_SEL_CHANNEL1: u8 = 0b01 << 6;
const PIT_SEL_CHANNEL2: u8 = 0b10 << 6;
const PIT_SEL_READBACK: u8 = 0b11 << 6;

const PIT_LATCH_COUNT: u8 = 0b00 << 4;
const PIT_ACCESS_LO: u8 = 0b01 << 4;
const PIT_ACCESS_HI: u8 = 0b10 << 4;
const PIT_ACCESS_LO_HI: u8 = 0b11 << 4;

const PIT_MODE0: u8 = 0b000 << 1;
const PIT_MODE1: u8 = 0b001 << 1;
const PIT_MODE2: u8 = 0b010 << 1;
const PIT_MODE3: u8 = 0b011 << 1;
const PIT_MODE4: u8 = 0b100 << 1;
const PIT_MODE5: u8 = 0b101 << 1;
const PIT_MODE2_2: u8 = 0b110 << 1;
const PIT_MODE3_2: u8 = 0b111 << 1;

const PIT_MODE_BIN: u8 = 0;
const PIT_MODE_BCD: u8 = 1;

const TIMER_BASE_FREQUENCY: usize = 1193182;

const TIMER_IRQ: u8 = 0;

extern "C" {
    fn __pit_timer_interrupt();
}

const TIMER_FREQUENCY: usize = 1000;

pub fn init() -> bool {
    assert!(TIMER_FREQUENCY >= 19 && TIMER_FREQUENCY <= TIMER_BASE_FREQUENCY);
    let reload_value: u16 = if TIMER_FREQUENCY == 0 {
        u16::MAX
    } else {
        (TIMER_BASE_FREQUENCY / TIMER_FREQUENCY) as u16
    };

    outb(
        PIT_MODE_CMD_REG,
        PIT_SEL_CHANNEL0 | PIT_ACCESS_LO_HI | PIT_MODE2 | PIT_MODE_BIN,
    );

    outb(PIT_CHANNEL0_DATA, (reload_value & 0xff) as u8);
    outb(PIT_CHANNEL0_DATA, (reload_value >> 8) as u8);

    pic::install_irq_handler(TIMER_IRQ, __pit_timer_interrupt as u64);
    println!("timer initialized, running at {}Hz", TIMER_FREQUENCY);
    enable();

    true
}

#[no_mangle]
fn pit_timer_interrupt(interrupt_regs: &mut InterruptRegisters) {
    // FIXME: figure out a better way to calculate how many milliseconds we want to advance the clock
    let ms_passed = 1000 / TIMER_FREQUENCY;
    time::advance(ms_passed as u64);

    SCHEDULER.tick(interrupt_regs);
    send_irq_eoi(TIMER_IRQ);
}

pub fn enable() {
    clear_irq(TIMER_IRQ);
}

pub fn disable() {
    set_irq(TIMER_IRQ);
}
