use core::str::from_utf8;

use alloc::{boxed::Box, string::String};

use crate::{
    framebuffer,
    fs::devfs::{self, DeviceOperations},
};

const ALTERNATE_TTY_DEVICE_MAJOR: u16 = 5;

struct Console {}

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
        framebuffer::draw_text(str, 0, 0);
        Ok(str.len())
    }
}

pub fn init() {
    devfs::register_devfs_node(&[String::from("console")], ALTERNATE_TTY_DEVICE_MAJOR, 1).unwrap();
    devfs::register_devfs_node_operations(ALTERNATE_TTY_DEVICE_MAJOR, Box::new(Console {}))
        .unwrap();
}
