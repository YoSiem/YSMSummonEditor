//! Bottom bar: last status message, connection and asset index state.

use eframe::egui::{self, Align, Layout, Margin, RichText, Ui};

use super::ViewContext;
use crate::icons;
use crate::state::ConnectionStatus;
use crate::theme::color;
use crate::widgets;

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let state = view.state;
    let frame = egui::Frame::new().fill(color::PANEL_BG).inner_margin(Margin::symmetric(12, 5));
    egui::Panel::bottom("status-bar").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(widgets::muted(state.status.as_str()));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(widgets::muted(concat!("v", env!("CARGO_PKG_VERSION"))).small());
                ui.separator();
                let (icon, text, tint, detail) = match &state.connection {
                    ConnectionStatus::Connected(server) => {
                        (icons::CONNECTION, state.settings.database.display_target(), color::SUCCESS, server.clone())
                    }
                    ConnectionStatus::Demo => (
                        icons::DATABASE,
                        "Demo data".to_owned(),
                        color::WARNING,
                        "Built-in sample data; start without --demo to use a database.".to_owned(),
                    ),
                    ConnectionStatus::Connecting => {
                        (icons::CONNECTION, "Connecting…".to_owned(), color::TEXT_MUTED, String::new())
                    }
                    ConnectionStatus::Failed(error) => {
                        (icons::NO_CONNECTION, "Connection failed".to_owned(), color::DANGER, error.clone())
                    }
                    ConnectionStatus::NotConfigured => (
                        icons::NO_CONNECTION,
                        "Database not configured".to_owned(),
                        color::WARNING,
                        "Open Settings to set up the connection.".to_owned(),
                    ),
                };
                let connection = ui.label(RichText::new(format!("{icon}  {text}")).color(tint));
                if !detail.is_empty() {
                    connection.on_hover_text(detail);
                }
                if state.connection == ConnectionStatus::Connecting {
                    ui.spinner();
                }

                ui.separator();
                match state.asset_count {
                    Some(count) => {
                        let tint = if count == 0 { color::WARNING } else { color::TEXT_MUTED };
                        ui.label(RichText::new(format!("{}  {count} images", icons::IMAGES)).color(tint))
                            .on_hover_text(format!("Dump folder: {}", state.settings.dump_folder_path));
                    }
                    None => {
                        ui.label(widgets::muted(format!("{}  Indexing images…", icons::IMAGES)));
                        ui.spinner();
                    }
                }
            });
        });
    });
}
