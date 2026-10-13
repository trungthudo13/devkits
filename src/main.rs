#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ui;
fn main() -> eframe::Result {
    ui::run()
}
