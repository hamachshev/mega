use std::env::args;

use crate::editor::Editor;

mod command;
mod editor;
mod keys;
mod terminal;

fn main() {
    terminal::make_raw().unwrap();

    let mut editor = Editor::new();

    if let Some(path) = args().nth(1) {
        editor.open(path.into()).unwrap();
    }
    editor.start();

    terminal::disable_raw().unwrap();
}
