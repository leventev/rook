use alloc::fmt;
use spin::Mutex;

use crate::arch::x86_64;

// TODO: use a mutex or something?
static mut BOOT_TIME: u64 = 0;

#[derive(Clone, Copy)]
pub struct Time {
    seconds: u64,
    milliseconds: u64, // between 0 and 1000
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.seconds + self.milliseconds / 1000;
        let milliseconds = self.milliseconds % 1000;
        write!(f, "{: >6}.{:0>3}", seconds, milliseconds)
    }
}

static SYSTEM_CLOCK: Mutex<Time> = Mutex::new(Time {
    seconds: 0,
    milliseconds: 0,
});

pub fn init(boot_time: u64) {
    unsafe {
        BOOT_TIME = boot_time;
    }
}

pub fn advance(ms: u64) {
    let mut clock = SYSTEM_CLOCK.lock();
    clock.milliseconds += ms;
    clock.seconds += clock.milliseconds / 1000;
    clock.milliseconds = clock.milliseconds % 1000;
}

// TODO: consider returning a reference
pub fn elapsed() -> Time {
    let interrupts_enabled = x86_64::interrupts_enabled();
    if interrupts_enabled {
        x86_64::disable_interrupts();
    }

    let time;
    {
        let clock = SYSTEM_CLOCK.lock();
        time = clock.clone();
    }

    if interrupts_enabled {
        x86_64::enable_interrupts();
    }

    time
}

pub fn global_time() -> Time {
    let elapsed = elapsed();
    Time {
        seconds: unsafe { BOOT_TIME } + elapsed.seconds,
        milliseconds: elapsed.milliseconds,
    }
}
