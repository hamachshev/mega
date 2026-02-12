use std::{
    io::{self, ErrorKind, stdin},
    os::fd::AsRawFd,
    sync::OnceLock,
};

use termios::{TCSAFLUSH, Termios, VMIN, VTIME, cfmakeraw, tcsetattr};

static ORIG_TERMIOS: OnceLock<Termios> = OnceLock::new();

pub fn make_raw() -> io::Result<()> {
    let mut termios = Termios::from_fd(stdin().as_raw_fd())?;
    let _ = ORIG_TERMIOS.set(termios.clone());

    termios.c_cc[VTIME] = 1;
    termios.c_cc[VMIN] = 0;

    cfmakeraw(&mut termios);

    tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &mut termios)?;
    Ok(())
}

pub fn disable_raw() -> io::Result<()> {
    let termios = ORIG_TERMIOS.get().ok_or(io::Error::new(
        ErrorKind::NotFound,
        "need to make raw first",
    ))?;
    tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &termios)?;
    Ok(())
}

pub fn size() -> io::Result<(u16, u16)> {
    let winsize = rustix::termios::tcgetwinsize(stdin())?;
    Ok((winsize.ws_col, winsize.ws_row))
}
