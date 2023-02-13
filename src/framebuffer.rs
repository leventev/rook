use alloc::slice;
use spin::Mutex;

use crate::mm::VirtAddr;

#[derive(Debug, PartialEq)]
pub enum FramebufferMode {
    Text,
    Graphics,
}

#[derive(Debug)]
pub struct Framebuffer {
    buffer: VirtAddr,
    mode: FramebufferMode,
    width: usize,
    height: usize,
    bits_per_pixel: usize,
    pitch: usize,
}

unsafe impl Send for Framebuffer {}

impl Framebuffer {
    const fn new() -> Self {
        Framebuffer {
            buffer: VirtAddr::zero(),
            mode: FramebufferMode::Graphics,
            width: 0,
            height: 0,
            bits_per_pixel: 0,
            pitch: 0,
        }
    }

    fn size(&self) -> usize {
        self.pitch * self.height
    }

    #[inline]
    fn draw_pixel(&self, x: usize, y: usize, red: u8, green: u8, blue: u8) {
        // TODO: support bpp other than 32 bits
        let buff = unsafe { slice::from_raw_parts_mut(self.buffer.get() as *mut u8, self.size()) };
        let y_off = y * self.pitch;
        let x_off = x * (self.bits_per_pixel / 8);

        buff[y_off + x_off + 2] = red;
        buff[y_off + x_off + 1] = green;
        buff[y_off + x_off + 0] = blue;
    }
}

static FRAMEBUFFER: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());

pub fn init(
    buff_addr: VirtAddr,
    pixel_width: usize,
    pixel_height: usize,
    pitch: usize,
    bits_per_pixel: usize,
) {
    assert_eq!(bits_per_pixel, 32, "bpp not supported");

    let mut fb = FRAMEBUFFER.lock();
    fb.buffer = buff_addr;
    fb.width = pixel_width;
    fb.pitch = pitch;
    fb.height = pixel_height;
    fb.bits_per_pixel = bits_per_pixel;
}

pub fn test() {
    let fb = FRAMEBUFFER.lock();
    for i in 0..fb.width {
        fb.draw_pixel(i, 0, 255, 0, 0);
    }
}

pub fn draw_pixel(x: usize, y: usize, red: u8, green: u8, blue: u8) {
    let fb = FRAMEBUFFER.lock();
    assert!(fb.mode == FramebufferMode::Graphics);
    fb.draw_pixel(x, y, red, green, blue);
}
