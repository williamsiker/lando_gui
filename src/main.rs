#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod lando;
mod models;
mod service_ui;
mod database_ui;
mod appserver_ui;
mod node_ui;

use app::LandoGui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Lando GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(LandoGui::new(cc)))),
    )
}