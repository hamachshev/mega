use core::panic;
use std::io::{Read, Write, stdin, stdout};

use crossterm::terminal;

use crate::command;

pub struct Editor {
    rows: u16,
    cols: u16,
    buffer: Vec<u8>,
    cx: u32,
    cy: u32,
}

impl Editor {
    pub fn new() -> Self {
        let size = terminal::size().expect("couldnt get size of terminal window");
        Editor {
            rows: size.1,
            cols: size.0,
            buffer: Vec::new(),
            cx: 0,
            cy: 0,
        }
    }

    pub fn start(&mut self) {
        self.refresh_screen();
        self.process_keypress();
    }
    fn process_keypress(&mut self) {
        while let Some(c) = self.read_key() {
            match c {
                c if c == control_key('q') => break,
                c if c == 'a' || c == 'd' || c == 's' || c == 'w' => {
                    self.move_cursor(c);
                }
                _ => {}
            }
            self.refresh_screen();
        }
    }
    fn read_key(&self) -> Option<char> {
        let mut buf = [0u8; 1];
        stdin().read(&mut buf).ok()?;
        if buf[0] == b'\x1b' {
            let escape_code = [0u8; 2];
            stdin().read(&mut buf).ok()?;
            if escape_code[0] == b'[' {
                match escape_code[1] {
                    b'A' => return Some('w'),
                    b'B' => return Some('s'),
                    b'C' => return Some('a'),
                    b'D' => return Some('d'),
                    _ => {}
                }
            }
        }
        return Some(buf[0] as char);
    }
    fn draw_rows(&mut self) {
        for row in 0..self.rows {
            if row == self.rows / 3 {
                let msg = format!("Mega editor -- version {}", env!("CARGO_PKG_VERSION"));
                let msg_len = if msg.len() <= self.cols as usize {
                    msg.len()
                } else {
                    self.cols as usize
                };
                let mut msg_padding = (self.cols as usize - msg_len) / 2;

                if msg_padding != 0 {
                    self.buffer.push(b'~');
                    msg_padding -= 1;
                }

                for _ in 0..msg_padding {
                    self.buffer.push(b' ');
                }
                self.buffer.extend_from_slice(&msg[0..msg_len].as_bytes());
            } else {
                self.buffer.push(b'~');
            }

            self.buffer.extend_from_slice(command::CLEAR_REST_OF_LINE);

            if row < self.rows - 1 {
                self.buffer.extend_from_slice(b"\r\n");
            }
        }
    }

    fn refresh_screen(&mut self) {
        self.buffer.extend_from_slice(command::HIDE_CURSOR);
        self.buffer.extend_from_slice(command::MOVE_CURSOR_TOP_LEFT);

        self.draw_rows();

        self.buffer
            .extend_from_slice(command::move_cursor(self.cx + 1, self.cy + 1));
        self.buffer.extend_from_slice(command::SHOW_CURSOR);

        stdout().write_all(&self.buffer).unwrap();
        stdout().flush().unwrap()
    }

    fn move_cursor(&mut self, key: char) {
        match key {
            'a' => self.cy -= 1,
            'd' => self.cy += 1,
            'w' => self.cx -= 1,
            's' => self.cx += 1,
            _ => panic!("this should not happen"),
        }
    }

    fn clear_screen(&self) {
        stdout().write(command::CLEAR_SCREEN).unwrap();
        stdout().flush().unwrap();
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.clear_screen();
    }
}

const fn control_key(key: char) -> char {
    (key as u8 & 0x1f) as char
}
