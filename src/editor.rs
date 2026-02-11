use std::{
    io::{Read, Write, stdin, stdout},
    process::exit,
};

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};

pub struct Editor {
    rows: u16,
    cols: u16,
}

impl Editor {
    pub fn new() -> Self {
        let size = crossterm::terminal::size().expect("couldnt get size of terminal window");
        Editor {
            rows: size.0,
            cols: size.1,
        }
    }

    pub fn start(&mut self) {
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();

        for row in 0..self.rows {
            if row == self.rows - 1 {
                print!("~");
            } else {
                print!("~\r\n");
            }
        }
        stdout().flush().unwrap();
        for c in stdin().bytes().flatten().map(|b| b as char) {
            match c {
                c if c == 'q' => break,
                _ => {
                    print!("{}", c);
                    stdout().flush().unwrap();
                }
            }
        }
    }
}
