use core::fmt;

use crate::{drivers, sync::InterruptMutex, time};

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
                if self.newline {
                    self.newline = false;
                    self.write_fmt(format_args!("[{}]: ", time::elapsed()))?;
                }

                drivers::serial::write(c);

                if c == b'\n' {
                    self.newline = true;
                }
            }
        }

        Ok(())
    }
}

static WRITER: InterruptMutex<Writer> = InterruptMutex::new(Writer { newline: false });

pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    fmt::Write::write_fmt(&mut *writer, args).ok();
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::io::_print(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! println {
    ()          => { $crate::print!("\n"); };
    // On nightly, `format_args_nl!` could also be used.
    ($($t:tt)*) => { $crate::print!("{}\n", format_args!($($t)*)) };
}
