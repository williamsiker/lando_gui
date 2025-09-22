#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod ui;
mod core;

use models::app::LandoGui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Lando GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(LandoGui::new(cc)))),
    )
}
