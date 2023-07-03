use alloc::fmt;

use crate::sync::InterruptMutex;

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

static SYSTEM_CLOCK: InterruptMutex<Time> = InterruptMutex::new(Time {
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
    clock.milliseconds %= 1000;
}

// TODO: consider returning a reference
pub fn elapsed() -> Time {
    let clock = SYSTEM_CLOCK.lock();
    *clock
}

pub fn global_time() -> Time {
    let elapsed = elapsed();
    Time {
        seconds: unsafe { BOOT_TIME } + elapsed.seconds,
        milliseconds: elapsed.milliseconds,
    }
}
