// The console window is a nuisance on Windows for a GUI application, and a
// necessity when something goes wrong before the window appears. Hidden in
// release only.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    biblecompose_tauri_lib::run();
}
