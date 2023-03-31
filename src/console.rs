use core::str::from_utf8;

use alloc::{boxed::Box, string::String};

use crate::{
    framebuffer,
    fs::devfs::{self, DeviceOperations},
    posix::{Termios, ECHO, ICANON, ISIG, NCCS, TCGETS, TCSETS, TIOCGPGRP, TIOCSPGRP},
};

const ALTERNATE_TTY_DEVICE_MAJOR: u16 = 5;

struct Console {
    termios: Termios,
    controlling_process_group: usize,
    y: usize,
}

impl DeviceOperations for Console {
    fn read(
        &mut self,
        _minor: u16,
        _offset: usize,
        _buff: &mut [u8],
        _size: usize,
    ) -> Result<usize, crate::fs::FileSystemError> {
        todo!()
    }

    fn write(
        &mut self,
        _minor: u16,
        _offset: usize,
        buff: &[u8],
        _size: usize,
    ) -> Result<usize, crate::fs::FileSystemError> {
        let str = from_utf8(buff).unwrap();
        framebuffer::draw_text(str, 0, self.y);
        self.y += 1;

        Ok(str.len())
    }

    fn ioctl(
        &mut self,
        _minor: u16,
        req: usize,
        arg: usize,
    ) -> Result<usize, crate::fs::FileSystemError> {
        match req {
            TCGETS => {
                let ptr = arg as *mut Termios;
                unsafe {
                    ptr.write(self.termios);
                }
                println!("TCGETS")
            }
            TCSETS => {
                let ptr = arg as *const Termios;
                self.termios = unsafe { ptr.read() };
                println!("TCSETS")
            }
            TIOCGPGRP => {
                let ptr = arg as *mut u32;
                println!("TIOCGPGRP {:?}", ptr);
                unsafe {
                    ptr.write(self.controlling_process_group as u32);
                }
            }
            TIOCSPGRP => {
                let ptr = arg as *const u32;
                self.controlling_process_group = unsafe { ptr.read() } as usize;
                println!("TIOCSPGRP");
            }
            _ => panic!("unimplemented ioctl req {}", req),
        }

        Ok(0)
    }
}

pub fn init() {
    devfs::register_devfs_node(&[String::from("console")], ALTERNATE_TTY_DEVICE_MAJOR, 1).unwrap();
    devfs::register_devfs_node_operations(
        ALTERNATE_TTY_DEVICE_MAJOR,
        Box::new(Console {
            termios: Termios {
                c_iflag: 0,
                c_oflag: 0,
                c_cflag: 0,
                c_lflag: (ISIG | ICANON | ECHO) as u32,
                c_cc: [0; NCCS],
            },
            controlling_process_group: 1,
            y: 0,
        }),
    )
    .unwrap();
}
