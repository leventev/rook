use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;

use crate::{
    drivers::ps2::{
        self,
        keyboard::{KeyEvent, PS2KeyboardEventHandler, PS2_KEY_BACKSPACE},
    },
    framebuffer,
    fs::{
        devfs::{self, DevFsDevice},
        errors::{FsIoctlError, FsReadError, FsStatError, FsWriteError},
        path::Path,
    },
    posix::{
        termios::{
            Termios, Winsize, ECHO, ICANON, ISIG, NCCS, TCGETS, TCSETS, TIOCGPGRP, TIOCGWINSZ,
            TIOCSPGRP, TIOCSWINSZ,
        },
        S_IFCHR,
    },
    sync::InterruptMutex,
};

const ALTERNATE_TTY_DEVICE_MAJOR: u16 = 5;

struct StdinBuffer {
    current_line: Vec<u8>,
    buffer: Vec<u8>,
    buffer_idx: usize,
}

struct Terminal {
    width: usize,
    height: usize,
    x: usize,
    y: usize,
}

struct ConsoleState {
    termios: Termios,
    controlling_process_group: usize,
}

struct Console {
    state: Mutex<ConsoleState>,
    stdin_buffer: InterruptMutex<StdinBuffer>,
    terminal: Mutex<Terminal>,
}

impl StdinBuffer {
    /// Creates a new StdinBuffer instance
    fn new() -> Self {
        StdinBuffer {
            current_line: Vec::new(),
            buffer: Vec::new(),
            buffer_idx: 0,
        }
    }

    /// Appends the current line to the end of the buffer, clears the current line
    fn add_line_to_buffer(&mut self) {
        if self.current_line.is_empty() {
            return;
        }

        self.buffer.append(&mut self.current_line);
    }

    /// Adds a char to the end of the current line
    fn add_char_to_line(&mut self, ch: u8) {
        assert!(ch != 0);
        self.current_line.push(ch);

        if ch == b'\n' {
            self.add_line_to_buffer();
        }
    }

    /// Removes a char from the end of the current line, returns whether a char was removed
    fn remove_char_from_end(&mut self) -> bool {
        if self.current_line.is_empty() {
            return false;
        }

        self.current_line.remove(self.current_line.len() - 1);
        true
    }

    /// Moves bytes from the beginning of the buffer to another buffer,
    /// then moves the remaining bytes to the front
    fn move_to_other_buffer(&mut self, size: usize, dst: &mut [u8]) {
        assert!(size <= self.buffer.len());
        assert!(size <= dst.len());

        let src = &self.buffer[..size];
        let dst = &mut dst[..size];

        dst.copy_from_slice(src);

        self.buffer.drain(..size);
    }
}

impl Terminal {
    /// Creates a new Terminal instance
    fn new() -> Self {
        Terminal {
            x: 0,
            y: 0,
            width: 80,
            height: 25,
        }
    }

    /// Writes a char to the screen, jumps to the start of the next line
    /// if the end of the line is reached or a newline char is written
    fn write_char(&mut self, ch: u8) {
        if ch == b'\n' {
            self.x = 0;
            self.y += 1;
        } else {
            framebuffer::draw_character(ch as char, self.x, self.y, true);

            self.x += 1;
            if self.x >= self.width {
                self.y += 1;
                self.x = 0;
            }
        }

        // TODO: scrolling
    }

    /// Remove the char at the cursor and moves the cursor back by 1
    fn backspace(&mut self) {
        if self.x == 0 && self.y > 0 {
            self.y -= 1;
        } else if self.x > 0 {
            self.x -= 1;
        }
        framebuffer::draw_character(' ', self.x, self.y, true);
    }
}

impl ConsoleState {
    fn new() -> Self {
        ConsoleState {
            termios: Termios {
                c_iflag: 0,
                c_oflag: 0,
                c_cflag: 0,
                c_lflag: (ISIG | ICANON | ECHO) as u32,
                c_cc: [0; NCCS],
            },
            controlling_process_group: 1,
        }
    }
}

impl DevFsDevice for Console {
    fn read(&self, _minor: u16, _off: usize, buff: &mut [u8]) -> Result<usize, FsReadError> {
        loop {
            let buffer = self.stdin_buffer.lock();
            if !buffer.buffer.is_empty() {
                break;
            }
        }

        // FIXME: interrupt locking because an keyboard interrupt could cause a deadlock here
        let mut stdin_buffer = self.stdin_buffer.lock();
        let bytes_to_read = usize::min(buff.len(), stdin_buffer.buffer.len());

        stdin_buffer.move_to_other_buffer(bytes_to_read, buff);

        Ok(bytes_to_read)
    }

    fn write(&self, _minor: u16, _off: usize, buff: &[u8]) -> Result<usize, FsWriteError> {
        let mut terminal = self.terminal.lock();
        for &ch in buff {
            terminal.write_char(ch);
        }

        Ok(buff.len())
    }

    fn ioctl(&self, _minor: u16, req: usize, arg: usize) -> Result<usize, FsIoctlError> {
        let mut state = self.state.lock();
        match req {
            TCGETS => {
                let ptr = arg as *mut Termios;
                unsafe {
                    ptr.write(state.termios);
                }
            }
            TCSETS => {
                let ptr = arg as *const Termios;
                state.termios = unsafe { ptr.read() };
            }
            TIOCGPGRP => {
                let ptr = arg as *mut u32;
                unsafe {
                    ptr.write(state.controlling_process_group as u32);
                }
            }
            TIOCSPGRP => {
                let ptr = arg as *const u32;
                state.controlling_process_group = unsafe { ptr.read() } as usize;
            }
            TIOCGWINSZ => {
                let terminal = self.terminal.lock();
                let ptr = arg as *mut Winsize;
                unsafe {
                    (*ptr).ws_col = terminal.width as u16;
                    (*ptr).ws_row = terminal.height as u16;
                }
            }
            TIOCSWINSZ => {
                let mut terminal = self.terminal.lock();
                let ptr = arg as *const Winsize;
                unsafe {
                    terminal.width = (*ptr).ws_col as usize;
                    terminal.height = (*ptr).ws_row as usize;
                }
            }
            _ => panic!("unimplemented ioctl req {}", req),
        }

        Ok(0)
    }

    fn stat(&self, _minor: u16, stat_buf: &mut crate::posix::Stat) -> Result<(), FsStatError> {
        // TODO
        stat_buf.st_blksize = 4096;
        stat_buf.st_blocks = 0;
        stat_buf.st_size = 0;
        stat_buf.st_dev = 0;
        stat_buf.st_gid = 0;
        stat_buf.st_uid = 0;
        stat_buf.st_nlink = 1;
        stat_buf.st_mode = S_IFCHR | 0o666;

        Ok(())
    }
}

impl PS2KeyboardEventHandler for Console {
    fn key_event(&self, ev: KeyEvent) {
        if !ev.pressed {
            return;
        }

        let mut terminal = self.terminal.lock();
        let mut buff = self.stdin_buffer.lock();

        if ev.key == PS2_KEY_BACKSPACE {
            let not_empty = buff.remove_char_from_end();
            if not_empty {
                terminal.backspace();
            }
        } else if ev.ch != 0 {
            buff.add_char_to_line(ev.ch);
            terminal.write_char(ev.ch);
        }
    }
}

pub fn init() {
    let con = Arc::new(Console {
        state: Mutex::new(ConsoleState::new()),
        stdin_buffer: InterruptMutex::new(StdinBuffer::new()),
        terminal: Mutex::new(Terminal::new()),
    });

    devfs::register_devfs_node(
        Path::new("/console").unwrap(),
        ALTERNATE_TTY_DEVICE_MAJOR,
        1,
    )
    .unwrap();
    devfs::register_devfs_node_operations(ALTERNATE_TTY_DEVICE_MAJOR, con.clone()).unwrap();

    ps2::keyboard::set_key_event_handler(Some(con));
}
