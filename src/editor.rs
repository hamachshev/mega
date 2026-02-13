use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write, stdin, stdout},
    path::Path,
};

use crate::{command, editor, terminal};

pub struct Editor {
    rows: u16,
    cols: u16,
    row_offset: usize,
    buffer: Vec<u8>,
    cx: u32,
    cy: u32,
    lines: Vec<String>,
    num_lines: usize,
}

impl Editor {
    pub fn new() -> Self {
        let size = terminal::size().expect("couldnt get size of terminal window");
        Editor {
            rows: size.1,
            cols: size.0,
            row_offset: 0,
            buffer: Vec::new(),
            cx: 0,
            cy: 0,
            lines: Vec::new(),
            num_lines: 0,
        }
    }

    pub fn start(&mut self) {
        self.refresh_screen();
        self.process_keypress();
    }
    pub fn open(&mut self, filename: &Path) -> io::Result<()> {
        let file = File::open(filename)?;
        let bufread = BufReader::new(file);

        let lines: Vec<String> = bufread.lines().flatten().collect();
        self.num_lines = lines.len();
        self.lines = lines;

        return Ok(());
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
            let line_in_file = row as usize + self.row_offset;

            if line_in_file < self.lines.len() {
                //we still have lines to print
                let line = &self.lines[line_in_file];
                let len = line.len().min((self.cols - 1) as usize);
                self.buffer.extend_from_slice(&line.as_bytes()[..len]);
            } else {
                if self.num_lines == 0 && row == self.rows / 3 {
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
            }

            self.buffer.extend_from_slice(command::CLEAR_REST_OF_LINE);

            if row < self.rows - 1 {
                self.buffer.extend_from_slice(b"\r\n");
            }
        }
    }
    fn scroll(&mut self) {
        if (self.cy as usize) < self.row_offset {
            self.row_offset = self.cy as usize
        }
        if self.cy as usize >= self.row_offset as usize + self.rows as usize {
            self.row_offset = self.cy as usize - self.rows as usize + 1
        }
    }

    fn refresh_screen(&mut self) {
        self.scroll();

        self.buffer.extend_from_slice(command::HIDE_CURSOR);
        self.buffer.extend_from_slice(command::MOVE_CURSOR_TOP_LEFT);

        self.draw_rows();

        self.buffer.extend_from_slice(command::move_cursor(
            (self.cy as usize - self.row_offset + 1) as u32,
            self.cx + 1,
        ));
        self.buffer.extend_from_slice(command::SHOW_CURSOR);

        stdout().write_all(&self.buffer).unwrap();
        self.buffer.clear();
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
                if self.cy < self.lines.len() as u32 {
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
        stdout().write(command::MOVE_CURSOR_TOP_LEFT).unwrap();
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
