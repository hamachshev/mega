use std::io::{Read, Write, stdin, stdout};

use crate::{command, editor, terminal};

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
                c if c == Key::Char('q').control() => break,
                Key::Special(EscapeSeq::UpArrow)
                | Key::Special(EscapeSeq::DownArrow)
                | Key::Special(EscapeSeq::RightArrow)
                | Key::Special(EscapeSeq::LeftArrow) => self.move_cursor(c),
                Key::Special(EscapeSeq::Home) => {
                    self.cx = 0;
                }

                Key::Special(EscapeSeq::End) => {
                    self.cx = self.cols as u32 - 1;
                }
                Key::Special(EscapeSeq::PageUp) | Key::Special(EscapeSeq::PageDown) => {
                    for _ in 0..self.rows {
                        self.move_cursor(if c == Key::Special(EscapeSeq::PageUp) {
                            Key::Special(EscapeSeq::UpArrow)
                        } else {
                            Key::Special(EscapeSeq::DownArrow)
                        });
                    }
                }
                _ => {}
            }
            self.refresh_screen();
        }
    }
    fn read_key(&self) -> Option<editor::Key> {
        let mut buf = [0u8; 1];
        stdin().read(&mut buf).ok()?;
        if buf[0] == b'\x1b' {
            let mut escape_code = [0u8; 3];
            stdin().read(&mut escape_code).ok()?;
            match escape_code[0] {
                b'[' => match escape_code[1] {
                    b'A' => return Some(Key::Special(EscapeSeq::UpArrow)),
                    b'B' => return Some(Key::Special(EscapeSeq::DownArrow)),
                    b'C' => return Some(Key::Special(EscapeSeq::RightArrow)),
                    b'D' => return Some(Key::Special(EscapeSeq::LeftArrow)),
                    b'H' => return Some(Key::Special(EscapeSeq::Home)),
                    b'F' => return Some(Key::Special(EscapeSeq::End)),
                    c => {
                        if escape_code[2] == b'~' {
                            match c {
                                b'1' => return Some(Key::Special(EscapeSeq::Home)),
                                b'3' => return Some(Key::Special(EscapeSeq::Delete)),
                                b'4' => return Some(Key::Special(EscapeSeq::End)),
                                b'5' => return Some(Key::Special(EscapeSeq::PageUp)),
                                b'6' => return Some(Key::Special(EscapeSeq::PageDown)),
                                b'7' => return Some(Key::Special(EscapeSeq::Home)),
                                b'8' => return Some(Key::Special(EscapeSeq::End)),
                                _ => {}
                            }
                        }
                    }
                },
                b'O' => match escape_code[1] {
                    b'H' => return Some(Key::Special(EscapeSeq::Home)),
                    b'F' => return Some(Key::Special(EscapeSeq::End)),
                    _ => {}
                },
                _ => {}
            }
        }
        return Some(Key::Char(buf[0] as char));
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
            .extend_from_slice(command::move_cursor(self.cy + 1, self.cx + 1));
        self.buffer.extend_from_slice(command::SHOW_CURSOR);

        stdout().write_all(&self.buffer).unwrap();
        stdout().flush().unwrap()
    }

    fn move_cursor(&mut self, key: Key) {
        match key {
            Key::Special(EscapeSeq::UpArrow) => {
                if self.cy > 0 {
                    self.cy -= 1
                }
            }
            Key::Special(EscapeSeq::DownArrow) => {
                if self.cy < self.rows as u32 {
                    self.cy += 1
                }
            }
            Key::Special(EscapeSeq::RightArrow) => {
                if self.cx < self.cols as u32 {
                    self.cx += 1
                }
            }
            Key::Special(EscapeSeq::LeftArrow) => {
                if self.cx > 0 {
                    self.cx -= 1
                }
            }
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

#[derive(PartialEq, Eq, Debug)]
enum Key {
    Char(char),
    Special(EscapeSeq),
}

impl Key {
    fn control(self) -> Self {
        match self {
            Key::Char(c) => Key::Char((c as u8 & 0x1f) as char),
            Key::Special(_) => self,
        }
    }
}
#[derive(PartialEq, Eq, Debug)]
enum EscapeSeq {
    RightArrow,
    LeftArrow,
    UpArrow,
    DownArrow,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
}
