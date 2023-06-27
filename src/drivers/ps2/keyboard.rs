use alloc::sync::Arc;
use bitflags::bitflags;
use spin::Mutex;

use crate::arch::x86_64::pic::send_irq_eoi;

use super::{controller::read_data_buffer, FIRST_PORT_IRQ};

bitflags! {
    pub struct KeyModifiers: u8 {
        const MOD_SHIFT = 1 << 0;
        const MOD_CTRL = 1 << 1;
        const MOD_ALT = 1 << 2;
        const MOD_SUPER = 1 << 3;
        const MOD_CAPSLOCK = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub scancode: u8,
    pub key: u8,
    pub ch: u8,
    pub pressed: bool,
    pub modifiers: KeyModifiers,
}

pub trait PS2KeyboardEventHandler {
    fn key_event(&self, ev: KeyEvent);
}

struct PS2Keyboard {
    extended_mode: bool,
    keys: [bool; 256],
    modifiers: KeyModifiers,
    key_event_handler: Option<Arc<dyn PS2KeyboardEventHandler>>,
}

unsafe impl Send for PS2Keyboard {}
unsafe impl Sync for PS2Keyboard {}

static KEYBOARD: Mutex<PS2Keyboard> = Mutex::new(PS2Keyboard {
    extended_mode: false,
    keys: [false; 256],
    modifiers: KeyModifiers::empty(),
    key_event_handler: None,
});

const SCANCODE_SET1: &[u8] = &[
    0, 0, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, b'\t', b'q',
    b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's', b'd',
    b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v', b'b',
    b'n', b'm', b',', b'.', b'/', 0, 0, 0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const SCANCODE_SET1_SHIFT: &[u8] = &[
    0, 0, b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 8, b'\t', b'Q',
    b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'{', b'}', b'\n', 0, b'A', b'S', b'D',
    b'F', b'G', b'H', b'J', b'K', b'L', b':', b'"', b'~', 0, b'|', b'Z', b'X', b'C', b'V', b'B',
    b'N', b'M', b'<', b'>', b'?', 0, 0, 0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const SCANCODE_SET1_EXTENDED: u8 = 0xE0;

const SCANCODE_SET1_LSHIFT: u8 = 0x2A;
const SCANCODE_SET1_RSHIFT: u8 = 0x36;

const SCANCODE_SET1_LALT: u8 = 0x38;
const SCANCODE_SET1_RALT: u8 = 0x38; // extended

const SCANCODE_SET1_LCTRL: u8 = 0x9D;
const SCANCODE_SET1_RCTRL: u8 = 0x1D; // extended

const SCANCODE_SET1_LSUPER: u8 = 0x5B; // extended
const SCANCODE_SET1_RSUPER: u8 = 0x5C; // extended

const SCANCODE_SET1_UP_ARROW: u8 = 0x48; // extended
const SCANCODE_SET1_LEFT_ARROW: u8 = 0x4B; // extended
const SCANCODE_SET1_RIGHT_ARROW: u8 = 0x4D; // extended
const SCANCODE_SET1_DOWN_ARROW: u8 = 0x50; // extended

const SCANCODE_SET1_HOME: u8 = 0x47; // extended
const SCANCODE_SET1_END: u8 = 0x4F; // extended

const SCANCODE_SET1_CAPSLOCK: u8 = 0x3A;

pub const PS2_KEY_NONE: u8 = 0x0;
pub const PS2_KEY_ESCAPE: u8 = 0x01;
pub const PS2_KEY_1: u8 = 0x02;
pub const PS2_KEY_2: u8 = 0x03;
pub const PS2_KEY_3: u8 = 0x04;
pub const PS2_KEY_4: u8 = 0x05;
pub const PS2_KEY_5: u8 = 0x06;
pub const PS2_KEY_6: u8 = 0x07;
pub const PS2_KEY_7: u8 = 0x08;
pub const PS2_KEY_8: u8 = 0x09;
pub const PS2_KEY_9: u8 = 0x0A;
pub const PS2_KEY_0: u8 = 0x0B;
pub const PS2_KEY_MINUS: u8 = 0x0C;
pub const PS2_KEY_EQUALS: u8 = 0x0D;
pub const PS2_KEY_BACKSPACE: u8 = 0xE;
pub const PS2_KEY_TAB: u8 = 0xF;
pub const PS2_KEY_Q: u8 = 0x10;
pub const PS2_KEY_W: u8 = 0x11;
pub const PS2_KEY_E: u8 = 0x12;
pub const PS2_KEY_R: u8 = 0x13;
pub const PS2_KEY_T: u8 = 0x14;
pub const PS2_KEY_Y: u8 = 0x15;
pub const PS2_KEY_U: u8 = 0x16;
pub const PS2_KEY_I: u8 = 0x17;
pub const PS2_KEY_O: u8 = 0x18;
pub const PS2_KEY_P: u8 = 0x19;
pub const PS2_KEY_LEFT_BRACE: u8 = 0x1A;
pub const PS2_KEY_RIGHT_BRACE: u8 = 0x1B;
pub const PS2_KEY_ENTER: u8 = 0x1C;
pub const PS2_KEY_LEFT_CTRL: u8 = 0x1D;
pub const PS2_KEY_A: u8 = 0x1E;
pub const PS2_KEY_S: u8 = 0x1F;
pub const PS2_KEY_D: u8 = 0x20;
pub const PS2_KEY_F: u8 = 0x21;
pub const PS2_KEY_G: u8 = 0x22;
pub const PS2_KEY_H: u8 = 0x23;
pub const PS2_KEY_J: u8 = 0x24;
pub const PS2_KEY_K: u8 = 0x25;
pub const PS2_KEY_L: u8 = 0x26;
pub const PS2_KEY_SEMICOLON: u8 = 0x27;
pub const PS2_KEY_SINGLE_QUOTE: u8 = 0x28;
pub const PS2_KEY_BACKTICK: u8 = 0x29;
pub const PS2_KEY_LEFT_SHIFT: u8 = 0x2A;
pub const PS2_KEY_BACKSLASH: u8 = 0x2B;
pub const PS2_KEY_Z: u8 = 0x2C;
pub const PS2_KEY_X: u8 = 0x2D;
pub const PS2_KEY_C: u8 = 0x2E;
pub const PS2_KEY_V: u8 = 0x2F;
pub const PS2_KEY_B: u8 = 0x30;
pub const PS2_KEY_N: u8 = 0x31;
pub const PS2_KEY_M: u8 = 0x32;
pub const PS2_KEY_COMMA: u8 = 0x33;
pub const PS2_KEY_DOT: u8 = 0x34;
pub const PS2_KEY_SLASH: u8 = 0x35;
pub const PS2_KEY_RIGHT_SHIFT: u8 = 0x36;
pub const PS2_KEY_LEFT_ALT: u8 = 0x38;
pub const PS2_KEY_SPACE: u8 = 0x39;
pub const PS2_KEY_CAPSLOCK: u8 = 0x3A;

// TODO: function keys, etc...
// TODO: renumber

pub const PS2_KEY_LEFT_SUPER: u8 = 0x40;
pub const PS2_KEY_RIGHT_SUPER: u8 = 0x41;
pub const PS2_KEY_RIGHT_CTRL: u8 = 0x42;
pub const PS2_KEY_RIGHT_ALT: u8 = 0x43;
pub const PS2_KEY_UP_ARROW: u8 = 0x44;
pub const PS2_KEY_LEFT_ARROW: u8 = 0x45;
pub const PS2_KEY_DOWN_ARROW: u8 = 0x46;
pub const PS2_KEY_RIGHT_ARROW: u8 = 0x47;
pub const PS2_KEY_HOME: u8 = 0x48;
pub const PS2_KEY_END: u8 = 0x49;

impl PS2Keyboard {
    fn key_event(&mut self, scancode: u8) {
        if scancode == SCANCODE_SET1_EXTENDED {
            self.extended_mode = true;
            return;
        }

        let pressed = scancode < 0x80;
        let scancode = if pressed { scancode } else { scancode - 0x80 };

        let key: u8 = if self.extended_mode {
            self.extended_mode = false;
            match scancode {
                SCANCODE_SET1_LSHIFT => PS2_KEY_LEFT_SHIFT,
                SCANCODE_SET1_RSHIFT => PS2_KEY_RIGHT_SHIFT,
                SCANCODE_SET1_RALT => PS2_KEY_RIGHT_ALT,
                SCANCODE_SET1_RCTRL => PS2_KEY_RIGHT_CTRL,
                SCANCODE_SET1_LSUPER => PS2_KEY_LEFT_SUPER,
                SCANCODE_SET1_RSUPER => PS2_KEY_RIGHT_SUPER,
                SCANCODE_SET1_UP_ARROW => PS2_KEY_UP_ARROW,
                SCANCODE_SET1_LEFT_ARROW => PS2_KEY_LEFT_ARROW,
                SCANCODE_SET1_DOWN_ARROW => PS2_KEY_DOWN_ARROW,
                SCANCODE_SET1_RIGHT_ARROW => PS2_KEY_RIGHT_ARROW,
                SCANCODE_SET1_HOME => PS2_KEY_HOME,
                SCANCODE_SET1_END => PS2_KEY_END,
                _ => {
                    return;
                }
            }
        } else {
            PS2_KEY_NONE + scancode
        };

        self.keys[key as usize] = pressed;

        match key {
            PS2_KEY_LEFT_SHIFT | PS2_KEY_RIGHT_SHIFT => {
                let (lshift, rshift) = (
                    self.keys[PS2_KEY_LEFT_SHIFT as usize],
                    self.keys[PS2_KEY_RIGHT_SHIFT as usize],
                );
                self.modifiers.set(KeyModifiers::MOD_SHIFT, lshift | rshift);
            }
            PS2_KEY_LEFT_CTRL | PS2_KEY_RIGHT_CTRL => {
                let (lctrl, rctrl) = (
                    self.keys[PS2_KEY_LEFT_CTRL as usize],
                    self.keys[PS2_KEY_RIGHT_CTRL as usize],
                );
                self.modifiers.set(KeyModifiers::MOD_CTRL, lctrl | rctrl);
            }
            PS2_KEY_LEFT_ALT | PS2_KEY_RIGHT_ALT => {
                let (lalt, ralt) = (
                    self.keys[PS2_KEY_LEFT_ALT as usize],
                    self.keys[PS2_KEY_RIGHT_ALT as usize],
                );
                self.modifiers.set(KeyModifiers::MOD_ALT, lalt | ralt);
            }
            PS2_KEY_LEFT_SUPER | PS2_KEY_RIGHT_SUPER => {
                let (lsuper, rsuper) = (
                    self.keys[PS2_KEY_LEFT_SUPER as usize],
                    self.keys[PS2_KEY_RIGHT_SUPER as usize],
                );
                self.modifiers.set(KeyModifiers::MOD_ALT, lsuper | rsuper);
            }
            PS2_KEY_CAPSLOCK => {
                if pressed {
                    self.modifiers.toggle(KeyModifiers::MOD_CAPSLOCK);
                }
            }
            _ => (),
        }

        if let Some(handler) = &self.key_event_handler {
            let ev = KeyEvent {
                key,
                scancode,
                ch: self.get_ch_from_key(key),
                pressed,
                modifiers: self.modifiers,
            };
            handler.key_event(ev);
        }
    }

    fn get_ch_from_key(&self, key: u8) -> u8 {
        let mut shifted = self.modifiers.contains(KeyModifiers::MOD_CAPSLOCK);
        if self.modifiers.contains(KeyModifiers::MOD_SHIFT) {
            shifted = !shifted;
        }

        if shifted {
            SCANCODE_SET1_SHIFT[key as usize]
        } else {
            SCANCODE_SET1[key as usize]
        }
    }
}

#[no_mangle]
fn handle_key_event() {
    let scancode = read_data_buffer().unwrap();

    let mut keyboard = KEYBOARD.lock();
    keyboard.key_event(scancode);

    send_irq_eoi(FIRST_PORT_IRQ);
}

pub fn set_key_event_handler(event_handler: Option<Arc<dyn PS2KeyboardEventHandler>>) {
    let mut keyboard = KEYBOARD.lock();
    keyboard.key_event_handler = event_handler;
}
