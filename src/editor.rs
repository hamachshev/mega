use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write, stdin, stdout},
    path::Path,
    usize,
};

use crate::{command, editor, terminal};

pub struct Editor {
    rows: u16,
    cols: u16,
    row_offset: usize,
    col_offset: usize,
    buffer: Vec<u8>,
    cx: u32,
    cy: u32,
    lines: Vec<String>,
}

impl Editor {
    pub fn new() -> Self {
        let size = terminal::size().expect("couldnt get size of terminal window");
        Editor {
            rows: size.1,
            cols: size.0,
            row_offset: 0,
            col_offset: 0,
            buffer: Vec::new(),
            cx: 0,
            cy: 0,
            lines: Vec::new(),
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
                let len: isize = (line.len() as isize) - (self.col_offset as isize);

                let len = (len.max(0) as usize).min((self.cols - 1) as usize);
                if len > 0 {
                    // need this because even if the range below is 0, the compiler will
                    // still try to index from offset to offset == nothing, but that doesnt exist
                    // in the string which is why the len is 0
                    self.buffer.extend_from_slice(
                        &line[self.col_offset..self.col_offset + len].as_bytes(),
                    );
                }
            } else {
                if self.lines.len() == 0 && row == self.rows / 3 {
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
            self.row_offset = self.cy as usize;
        }
        if self.cy as usize >= self.row_offset as usize + self.rows as usize {
            // technically, i think we can just add one to the offset because we are only moving
            // one step at a time, but i think we do this bc we will scroll with page up and down,
            // and that could be more than just 1 step at a time, so recalc based on cursor pointer
            // in the file
            self.row_offset = self.cy as usize - self.rows as usize + 1;
        }

        if (self.cx as usize) < self.col_offset {
            self.col_offset = self.cx as usize;
        }
        if self.cx as usize >= self.cols as usize + self.col_offset {
            self.col_offset = self.cx as usize - self.cols as usize + 1;
        }
    }

    fn refresh_screen(&mut self) {
        self.scroll();

        self.buffer.extend_from_slice(command::HIDE_CURSOR);
        self.buffer.extend_from_slice(command::MOVE_CURSOR_TOP_LEFT);

        self.draw_rows();

        self.buffer.extend_from_slice(command::move_cursor(
            (self.cy as usize - self.row_offset + 1) as u32,
            (self.cx as usize - self.col_offset + 1) as u32,
        ));
        self.buffer.extend_from_slice(command::SHOW_CURSOR);

        stdout().write_all(&self.buffer).unwrap();
        self.buffer.clear();
        stdout().flush().unwrap()
    }

    fn move_cursor(&mut self, key: Key) {
        // cy, could be one more than the file,
        // and if so there is not a corresponding line in lines, so will panic

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
                self.cx += 1;
                let curr_row = self.lines.get(self.cy as usize); // can be one more than lines, so
                // need to check
                if let Some(curr_row) = curr_row {
                    if (self.cx as usize) > curr_row.len() {
                        //went right on position end, apparently want to wrap down a line
                        // ie do not do this on the last line
                        self.cy += 1;
                        self.cx = 0;
                    }
                }
            }
            Key::Special(EscapeSeq::LeftArrow) => {
                if self.cx > 0 {
                    self.cx -= 1
                } else {
                    //went left on position 0, apparently want to wrap up a line
                    if self.cy > 0 {
                        // so no overflow
                        // do not do this on the first line
                        let prev_row = &self.lines[self.cy as usize - 1];
                        self.cy -= 1;
                        self.cx = (prev_row.len()) as u32;
                    }
                }
            }
            _ => panic!("this should not happen"),
        }
        //snap to end of line
        //need it down here bc may have adjusted the cy above
        let curr_row = self.lines.get(self.cy as usize);

        if let Some(curr_row) = curr_row {
            self.cx = self.cx.min(curr_row.len() as u32) // curr_row.len() is actually one more bc
        // 0 indexed so you can go one more than the length of the line
        } else {
            self.cx = 0;
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
