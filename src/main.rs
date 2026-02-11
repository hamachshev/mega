use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::editor::Editor;

mod editor;
fn main() {
    enable_raw_mode().unwrap();

    let mut editor = Editor::new();
    editor.start();

    disable_raw_mode().unwrap();
}
