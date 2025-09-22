#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod lando_commands;
mod models;
mod ui;

use app::LandoGui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Lando GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(LandoGui::new(cc)))),
    )
}
