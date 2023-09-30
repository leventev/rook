use alloc::{collections::BTreeMap, slice};
use spin::Mutex;

use crate::mm::VirtAddr;

mod font;

#[derive(Debug, PartialEq)]
pub enum FramebufferMode {
    Text,
    Graphics,
}

#[derive(Debug)]
/// Framebuffer
pub struct Framebuffer {
    /// Virtual address of the video memory
    buffer: VirtAddr,

    /// Current mode of the framebuffer
    mode: FramebufferMode,

    /// Width of the framebuffer in pixels
    width: usize,

    /// Height of the framebuffer in pixels
    height: usize,

    /// Number of bits per pixel(usually 32)
    bits_per_pixel: usize,

    /// Number of bytes per row
    pitch: usize,

    /// Number of columns that fit in the framebuffer
    text_columns: usize,

    /// Number of rows that fit in the framebuffer
    text_rows: usize,

    /// Width of the font in bits
    font_width: usize,

    /// Height of the font in bits
    font_height: usize,

    /// Number of glyphs available
    font_glyph_count: usize,

    /// Glyph size in bytes
    font_glyph_size: usize,

    /// Offset of the start of the glyph table in the PC Screen Font data
    font_glyph_table_start_offset: usize,

    /// Bytes per pixel row, for example a font with 9 bit height has a 2 byte pixel row
    font_pixel_row_size: usize,

    /// Unicode code-point to glyph translation table
    unicode_glyph_table: Option<BTreeMap<char, usize>>,
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
            font_width: 0,
            font_height: 0,
            font_glyph_count: 0,
            font_glyph_size: 0,
            font_glyph_table_start_offset: 0,
            font_pixel_row_size: 0,
            text_columns: 0,
            text_rows: 0,
            unicode_glyph_table: None,
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
        buff[y_off + x_off] = blue;
    }

    fn draw_glyph(&self, glyph_idx: usize, x: usize, y: usize, clear_background: bool) {
        let bitmap = self.get_glyph_bitmap(glyph_idx);

        let mut yy = y;

        for row in 0..self.font_height {
            let mut xx = x;
            let row_offset = row * self.font_pixel_row_size;
            let row_offset_end = row_offset + self.font_pixel_row_size;
            let row = &bitmap[row_offset..row_offset_end];

            for (col_byte, byte) in row.iter().enumerate().take(self.font_pixel_row_size) {
                let remaining_bits = self.font_height - col_byte * 8;
                let cols = usize::min(8, remaining_bits);

                for col in 0..cols {
                    let mask = 1 << (7 - col);
                    if byte & mask > 0 {
                        self.draw_pixel(xx, yy, 0xcf, 0xcf, 0xcf);
                    } else if clear_background {
                        self.draw_pixel(xx, yy, 0, 0, 0);
                    }
                    xx += 1;
                }
            }

            yy += 1;
        }
    }

    fn draw_character(&self, c: char, col: usize, row: usize, clear_background: bool) {
        let x = col * self.font_width;
        let y = row * self.font_height;
        let glyph = match &self.unicode_glyph_table {
            Some(table) => *table.get(&c).unwrap_or(&('?' as usize)),
            None => {
                let glyph = c as usize;
                if glyph >= self.font_glyph_count {
                    '?' as usize
                } else {
                    glyph
                }
            }
        };
        self.draw_glyph(glyph, x, y, clear_background);
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

pub fn init_font() {
    let mut fb = FRAMEBUFFER.lock();
    fb.init_font();
}

pub fn draw_pixel(x: usize, y: usize, red: u8, green: u8, blue: u8) {
    let fb = FRAMEBUFFER.lock();
    assert!(fb.mode == FramebufferMode::Graphics);
    fb.draw_pixel(x, y, red, green, blue);
}

pub fn draw_character(ch: char, col: usize, row: usize, clear_background: bool) {
    let fb = FRAMEBUFFER.lock();
    assert!(fb.mode == FramebufferMode::Graphics);
    fb.draw_character(ch, col, row, clear_background);
}
