use crate::editor::Editor;

mod command;
mod editor;
mod terminal;

fn main() {
    terminal::make_raw().unwrap();

    let mut editor = Editor::new();
    editor.start();

    terminal::disable_raw().unwrap();
}
