use crate::arch::x86_64::{
    disable_interrupts, enable_interrupts,
    pic::{self, clear_irq},
};

mod controller;
pub mod keyboard;

const FIRST_PORT_IRQ: u8 = 1;
const SECOND_PORT_IRQ: u8 = 12;

extern "C" {
    fn __ps2_first_interrupt();
}

pub fn init() -> bool {
    disable_interrupts();

    let res = match controller::init() {
        Ok(ports) => {
            match ports {
                (false, false) => false,
                (first, _second) => {
                    // TODO: don't assume the first port is the keyboard
                    assert!(first);

                    pic::install_irq_handler(FIRST_PORT_IRQ, __ps2_first_interrupt as usize as u64);
                    clear_irq(FIRST_PORT_IRQ);

                    true
                }
            }
        }
        Err(err) => {
            println!("PS2: initialization failed: {:?}", err);
            false
        }
    };

    enable_interrupts();

    res
}
