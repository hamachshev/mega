use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write, stdin, stdout},
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{command, editor, keys, terminal};

const MEGA_TAB_STOP: usize = 8;

pub struct Editor {
    rows: u16,
    cols: u16,
    row_offset: usize,
    col_offset: usize,
    buffer: Vec<u8>,
    cx: u32,
    cy: u32,
    rx: u32,
    lines: Vec<String>,
    render: Vec<String>,
    filename: Option<PathBuf>,
    status_msg: String,
    status_msg_time: Instant,
    dirty: bool,
}

impl Editor {
    pub fn new() -> Self {
        let size = terminal::size().expect("couldnt get size of terminal window");
        Editor {
            rows: size.1 - 2, // minus 1 for status bar, and minus 1 for message bar
            cols: size.0,
            row_offset: 0,
            col_offset: 0,
            buffer: Vec::new(),
            cx: 0,
            cy: 0,
            rx: 0,
            lines: Vec::new(),
            render: Vec::new(),
            filename: None,
            status_msg: String::new(),
            status_msg_time: Instant::now(),
            dirty: false,
        }
    }

    pub fn start(&mut self) {
        self.set_status_message("HELP: Ctrl-S to save | Ctrl-Q to quit");
        self.refresh_screen();
        self.process_keypress();
    }
    pub fn open(&mut self, filename: PathBuf) -> io::Result<()> {
        let file = File::open(&filename)?;
        self.filename = Some(filename);

        let bufread = BufReader::new(file);

        let lines: Vec<String> = bufread.lines().flatten().collect();
        self.render = lines
            .clone()
            .into_iter()
            .map(|line| {
                let mut idx: usize = 0;
                line.chars()
                    .map(|c| {
                        if c == '\t' {
                            let spaces_needed = (MEGA_TAB_STOP - (idx % MEGA_TAB_STOP)) as usize;
                            idx += spaces_needed;
                            " ".repeat(spaces_needed)
                        } else {
                            idx += 1;
                            c.to_string()
                        }
                    })
                    .collect()
            })
            .collect();
        self.lines = lines;

        return Ok(());
    }
    fn process_keypress(&mut self) {
        while let Some(c) = self.read_key() {
            match c {
                c if c == Key::Char('q').control() => break,
                c if c == Key::Char('l').control() => {}
                c if c == Key::Char('s').control() => match self.save() {
                    Ok(len) => {
                        self.set_status_message(&format!("{} bytes written to disk", len));
                    }
                    Err(error) => {
                        self.set_status_message(&format!("Can't save! IO error: {}", error));
                    }
                },
                Key::Special(EscapeSeq::UpArrow)
                | Key::Special(EscapeSeq::DownArrow)
                | Key::Special(EscapeSeq::RightArrow)
                | Key::Special(EscapeSeq::LeftArrow) => self.move_cursor(c),
                Key::Special(EscapeSeq::Home) => {
                    self.cx = 0;
                }

                Key::Special(EscapeSeq::End) => {
                    if self.cy < self.lines.len() as u32 {
                        self.cx = (&self.lines[self.cy as usize].len() - 1) as u32;
                    }
                }
                Key::Special(EscapeSeq::PageUp) => {
                    self.cy = self.row_offset as u32;
                    for _ in 0..self.rows {
                        self.move_cursor(Key::Special(EscapeSeq::UpArrow));
                    }
                }
                Key::Special(EscapeSeq::PageDown) => {
                    self.cy = (self.row_offset as u32) + (self.rows as u32) - 1;
                    if (self.cy as usize) > self.lines.len() {
                        self.cy = self.lines.len() as u32;
                    }
                    for _ in 0..self.rows {
                        self.move_cursor(Key::Special(EscapeSeq::DownArrow));
                    }
                }
                Key::Char(keys::BACKSPACE) | Key::Special(EscapeSeq::Delete) => {}
                c if c == Key::Char('h').control() => {}
                Key::Char(keys::ENTER) => {}
                Key::Char(c) => {
                    self.insert_char(c);
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

            if line_in_file < self.render.len() {
                //we still have lines to print
                let line = &self.render[line_in_file];
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
            } else if self.lines.len() == 0 && row == self.rows / 3 {
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

            self.buffer.extend_from_slice(b"\r\n");
        }
    }
    fn draw_status_bar(&mut self) {
        self.buffer.extend_from_slice(command::INVERTED_COLORS);
        let modified = if self.dirty { "(modified)" } else { "" };

        let status = match &self.filename {
            Some(filename) => match filename.to_str() {
                Some(filename) => {
                    format!("{} {} - {} lines", filename, modified, self.rows)
                }
                None => format!("[Non-Unicode file name] {} - {} lines", modified, self.rows),
            },
            None => {
                format!("No Name] - {} lines", self.rows)
            }
        };
        let len = status.len().min(self.cols as usize);
        self.buffer.extend_from_slice(&status[0..len].as_bytes());

        // line number
        let right_status = format!("{}/{}", self.cy + 1, self.lines.len());

        for _ in len..(self.cols as usize) - right_status.len() {
            self.buffer.push(b' ');
        }
        self.buffer.extend_from_slice(right_status.as_bytes());
        self.buffer.extend_from_slice(command::NORMAL_COLORS);
        self.buffer.extend_from_slice(b"\r\n");
    }
    fn draw_message_bar(&mut self) {
        self.buffer.extend_from_slice(command::CLEAR_REST_OF_LINE);
        if self.status_msg_time.elapsed() < Duration::new(5, 0) {
            self.buffer.extend_from_slice(self.status_msg.as_bytes());
        }
    }
    fn scroll(&mut self) {
        self.convert_cx_to_rx();

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

        if (self.rx as usize) < self.col_offset {
            self.col_offset = self.rx as usize;
        }
        if self.rx as usize >= self.cols as usize + self.col_offset {
            self.col_offset = self.rx as usize - self.cols as usize + 1;
        }
    }

    fn refresh_screen(&mut self) {
        self.scroll();

        self.buffer.extend_from_slice(command::HIDE_CURSOR);
        self.buffer.extend_from_slice(command::MOVE_CURSOR_TOP_LEFT);

        self.draw_rows();
        self.draw_status_bar();
        self.draw_message_bar();

        self.buffer.extend_from_slice(command::move_cursor(
            (self.cy as usize - self.row_offset + 1) as u32,
            (self.rx as usize - self.col_offset + 1) as u32,
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

    fn insert_char(&mut self, c: char) {
        if (self.cy as usize) > self.lines.len() {
            self.render.push(String::new());
            self.lines.push(String::new()); //add new line to render and lines buffers 
        }
        self.render[self.cy as usize].insert(self.rx as usize, c); // TODO: handle insert tabs
        self.lines[self.cy as usize].insert(self.cx as usize, c);

        self.dirty = true;
    }

    fn clear_screen(&self) {
        stdout().write(command::CLEAR_SCREEN).unwrap();
        stdout().write(command::MOVE_CURSOR_TOP_LEFT).unwrap();
        stdout().flush().unwrap();
    }
    fn convert_cx_to_rx(&mut self) {
        self.rx = 0;
        if (self.cy as usize) < self.lines.len() {
            let curr_row = &self.lines[self.cy as usize];

            for (i, c) in curr_row.chars().enumerate() {
                if i == self.cx as usize {
                    break;
                }

                self.rx += 1; //count char
                if c == '\t' {
                    let spaces_needed = (MEGA_TAB_STOP - (i % MEGA_TAB_STOP)) as u32;
                    self.rx += spaces_needed - 1; //already counted one for '\t'
                }
            }
        }
    }
    fn save(&mut self) -> io::Result<usize> {
        if let Some(filename) = &self.filename {
            let mut file = File::create(filename)?;
            let buf = self.lines.join("\n");
            file.write_all(buf.as_bytes())?;
            self.dirty = false;
            return Ok(buf.as_bytes().len());
        }
        self.dirty = false;
        Ok(0)
    }
    fn set_status_message(&mut self, msg: &str) {
        self.status_msg.clear();
        self.status_msg = msg.to_string();
        self.status_msg_time = Instant::now();
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
