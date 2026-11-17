//! YSM Summon Editor — desktop editor for Rappelz summon data.
//!
//! The UI is a thin layer: views emit [`action::Action`]s, the application
//! reduces them against [`state::AppState`] and sends [`worker::Command`]s to
//! a background thread that owns the database connection.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod action;
mod app;
mod dialogs;
mod icons;
mod state;
mod textures;
mod theme;
mod views;
mod widgets;
mod worker;

use eframe::egui;

const APP_TITLE: &str = "YSM Summon Editor";

fn main() -> eframe::Result {
    // `--demo` runs against built-in sample data instead of a database.
    let demo = std::env::args().skip(1).any(|arg| arg == "--demo");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_inner_size([1560.0, 980.0])
        .with_min_inner_size([1100.0, 700.0]);
    if let Some(icon) = theme::window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(APP_TITLE, options, Box::new(move |cc| Ok(Box::new(app::EditorApp::new(cc, demo)))))
}
