pub const CLEAR_SCREEN: &[u8] = b"\x1b[2J";
pub const CLEAR_REST_OF_LINE: &[u8] = b"\x1b[K";
pub const MOVE_CURSOR_TOP_LEFT: &[u8] = b"\x1b[H";
pub const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
pub const SHOW_CURSOR: &[u8] = b"\x1b[?25h";

pub fn move_cursor(row: u32, col: u32) -> &'static [u8] {
    format!("\x1b[{row};{col}H").leak().as_bytes()
}
