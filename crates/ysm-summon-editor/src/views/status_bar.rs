//! Bottom bar: last status message on the left; image index, connection and
//! version on the right.

use eframe::egui::{self, Align, Color32, Layout, Margin, RichText, Stroke, Ui};

use super::ViewContext;
use crate::action::Action;
use crate::icons;
use crate::state::ConnectionStatus;
use crate::theme::{self, color, space};
use crate::widgets;

const BAR_HEIGHT: f32 = 24.0;

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let state = view.state;
    let frame = egui::Frame::new().fill(color::WINDOW_BG).inner_margin(Margin::symmetric(space::MD as i8, 4));
    egui::Panel::bottom("status-bar").frame(frame).show_separator_line(false).show(ui, |ui| {
        let top = ui.max_rect().top() - 4.0;
        ui.painter().hline(ui.ctx().content_rect().x_range(), top, Stroke::new(1.0, color::BORDER));

        ui.horizontal(|ui| {
            ui.set_height(BAR_HEIGHT);
            ui.label(widgets::caption(state.status.as_str()));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = space::MD;
                ui.label(widgets::caption(concat!("v", env!("CARGO_PKG_VERSION"))));
                connection(ui, view);
                assets(ui, view);
            });
        });
    });
}

fn item(icon: &str, text: &str, tint: Color32) -> RichText {
    RichText::new(format!("{icon}  {text}")).size(theme::CAPTION_SIZE).color(tint)
}

fn connection(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let state = view.state;
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
        ConnectionStatus::Connecting => (icons::CONNECTION, "Connecting…".to_owned(), color::TEXT_MUTED, String::new()),
        ConnectionStatus::Failed(error) => {
            (icons::NO_CONNECTION, "Connection failed".to_owned(), color::DANGER, error.clone())
        }
        ConnectionStatus::Lost(error) => (
            icons::NO_CONNECTION,
            "Connection lost".to_owned(),
            color::DANGER,
            format!("{error}\n\nThe next request reconnects automatically."),
        ),
        ConnectionStatus::NotConfigured => (
            icons::NO_CONNECTION,
            "Database not configured".to_owned(),
            color::WARNING,
            "Open Settings to set up the connection.".to_owned(),
        ),
    };

    if state.connection.can_reconnect() {
        let reconnect = egui::Button::new(item(icons::REFRESH, "Reconnect", color::TEXT)).small();
        if ui.add(reconnect).clicked() {
            view.actions.push(Action::Reconnect);
        }
    }
    if state.connection == ConnectionStatus::Connecting {
        ui.add(egui::Spinner::new().size(theme::CAPTION_SIZE));
    }
    let label = ui.label(item(icon, &text, tint));
    if !detail.is_empty() {
        label.on_hover_text(detail);
    }
}

fn assets(ui: &mut Ui, view: &ViewContext<'_>) {
    let state = view.state;
    match state.asset_count {
        Some(count) => {
            let tint = if count == 0 { color::WARNING } else { color::TEXT_MUTED };
            ui.label(item(icons::IMAGES, &format!("{count} images"), tint))
                .on_hover_text(format!("Dump folder: {}", state.settings.dump_folder_path));
        }
        None => {
            ui.add(egui::Spinner::new().size(theme::CAPTION_SIZE));
            ui.label(item(icons::IMAGES, "Indexing images…", color::TEXT_MUTED));
        }
    }
}
