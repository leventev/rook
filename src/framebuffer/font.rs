use alloc::collections::BTreeMap;
use encode_unicode::Utf8Char;

use crate::utils;

use super::Framebuffer;

// https://wiki.osdev.org/PC_Screen_Font
// https://www.win.tue.nl/~aeb/linux/kbd/font-formats-1.html
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PSFHeader {
    magic: u32,
    version: u32,
    header_size: u32,
    flags: u32,
    glyph_count: u32,
    glyph_size: u32,
    height: u32,
    width: u32,
}

const PSF_MAGIC: u32 = 0x864ab572;
const FONT_DATA: &[u8] = core::include_bytes!("../default8x16.psfu");

const PSF_FLAGS_HAS_UNICODE_TABLE: u32 = 1 << 0;

impl Framebuffer {
    pub fn get_glyph_bitmap(&self, glyph_idx: usize) -> &'static [u8] {
        assert!(glyph_idx < self.font_glyph_count);
        let glyph_off = glyph_idx * self.font_glyph_size;

        let offset = self.font_glyph_table_start_offset + glyph_off;
        let end_offset = offset + self.font_glyph_size;

        &FONT_DATA[offset..end_offset]
    }

    pub fn init_font(&mut self) {
        let font_header = &(unsafe { FONT_DATA.align_to::<PSFHeader>().1 })[0];

        let magic = font_header.magic;
        assert_eq!(magic, PSF_MAGIC, "Console font magic number does not match");

        self.font_width = font_header.width as usize;
        self.font_height = font_header.height as usize;
        self.font_glyph_count = font_header.glyph_count as usize;
        self.font_glyph_size = font_header.glyph_size as usize;
        self.font_glyph_table_start_offset = font_header.header_size as usize;
        self.font_pixel_row_size = utils::div_and_ceil(self.font_width, 8);

        self.text_columns = self.width / self.font_width;
        self.text_rows = self.height / self.font_height;

        let has_unicode_table = font_header.flags & PSF_FLAGS_HAS_UNICODE_TABLE > 0;
        if !has_unicode_table {
            return;
        }

        self.unicode_glyph_table = Some(BTreeMap::new());

        let unicode_table_start_offset =
            self.font_glyph_table_start_offset + self.font_glyph_size * self.font_glyph_count;

        let unicode_table = &FONT_DATA[unicode_table_start_offset..];
        let mut idx = 0;
        let mut glyph = 0;

        let unicode_translation_table = &mut self.unicode_glyph_table.as_mut().unwrap();
        while idx < unicode_table.len() {
            let current = &unicode_table[idx..];
            if current[0] == 0xff {
                glyph += 1;
                idx += 1;
                continue;
            }

            let utf8_char = Utf8Char::from_slice_start(current).unwrap();
            let char = utf8_char.0.to_char();
            idx += utf8_char.1;

            unicode_translation_table.insert(char, glyph);
        }
    }
}
