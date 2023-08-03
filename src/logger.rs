use core::fmt;

use crate::{drivers, sync::InterruptMutex, time};

pub const USE_ANSI_CODES: bool = true;
pub const LOG_DEBUG: bool = true;

struct Writer {
    newline: bool,
}

unsafe impl Send for Writer {}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if cfg!(serial_module)
        /*&& drivers::is_loaded("serial")*/
        {
            for c in s.bytes() {
                drivers::serial::write(c);
            }
        }

        Ok(())
    }
}

static WRITER: InterruptMutex<Writer> = InterruptMutex::new(Writer { newline: false });

fn print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    fmt::Write::write_fmt(&mut *writer, args).ok();
}

pub fn print_log(name: &str, color: [u8; 3], args: fmt::Arguments) {
    let time = time::elapsed();

    if USE_ANSI_CODES {
        print(format_args_nl!(
            "{} \x1b[1m\x1b[38;2;{};{};{}m{}\x1b[0m: {}",
            time,
            color[0],
            color[1],
            color[2],
            name,
            args
        ));
    } else {
        print(format_args_nl!("{} {}: {}", time, name, args));
    };
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::logger::print_log("log", [40, 100, 190],  format_args!($($t)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => { $crate::logger::print_log("warn", [210, 200, 20], format_args!($($t)*)) };
}

#[macro_export]
macro_rules! debug {
    ($($t:tt)*) => {
        if $crate::logger::LOG_DEBUG {
            $crate::logger::print_log("dbg", [175, 100, 200], format_args!($($t)*))
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($t:tt)*) => { $crate::logger::print_log("error", [160, 15, 15], format_args!($($t)*)) };
}
