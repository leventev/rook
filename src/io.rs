use core::fmt;
use spin::Mutex;

use crate::{arch::x86_64, drivers};

struct Writer {}

unsafe impl Send for Writer {}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if cfg!(serial_module) /*&& drivers::is_loaded("serial")*/ {
            for c in s.bytes() {
                drivers::serial::write(c);
            }
        }

        Ok(())
    }
}

static WRITER: Mutex<Writer> = Mutex::new(Writer {});

pub fn _print(args: fmt::Arguments) {
    // NOTE: Locking needs to happen around `print_fmt`, not `print_str`, as the former
    // will call the latter potentially multiple times per invocation.

    let int_enabled = x86_64::interrupts_enabled();
    if int_enabled {
        x86_64::disable_interrupts();
    }

    {
        let mut writer = WRITER.lock();
        fmt::Write::write_fmt(&mut *writer, args).ok();
    }

    if int_enabled {
        x86_64::enable_interrupts();
    }
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::io::_print(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! println {
    ()          => { $crate::print!("\n"); };
    // On nightly, `format_args_nl!` could also be used.
    ($($t:tt)*) => { $crate::print!("[{}]: {}\n", $crate::time::elapsed(), format_args!($($t)*)); };
}
