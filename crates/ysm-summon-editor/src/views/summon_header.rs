//! Header card: the selected summon, its evolution chain and illustration.

use eframe::egui::{self, Align, Align2, CornerRadius, Layout, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use ysm_core::model::{EvolutionChain, STAGE_COUNT};

use super::ViewContext;
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color, space};
use crate::widgets;

/// Height of the card's content: name row, two text lines and the chain.
const CONTENT_HEIGHT: f32 = 152.0;
/// Width reserved for an arrow between two chain icons.
const ARROW_WIDTH: f32 = 24.0;
/// Room under the chain icons for the summon IDs.
const CHAIN_LABEL_HEIGHT: f32 = 20.0;

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    theme::card().show(ui, |ui| {
        ui.set_height(CONTENT_HEIGHT);
        ui.set_min_width(ui.available_width());

        let state = view.state;
        let Some(chain) = state.session.chain() else {
            placeholder(ui, state.chain_loading);
            return;
        };

        let illustration = view
            .textures
            .illustration(chain.stage3.illust_file_name.as_deref())
            .map(|texture| (texture.id(), texture.aspect_ratio()));
        let image_size = illustration.map(|(_, aspect)| Vec2::new(CONTENT_HEIGHT * aspect, CONTENT_HEIGHT));
        let image_width = image_size.map_or(0.0, |size| size.x + space::MD);

        ui.horizontal(|ui| {
            let text_width = (ui.available_width() - image_width).max(200.0);
            ui.allocate_ui_with_layout(Vec2::new(text_width, CONTENT_HEIGHT), Layout::top_down(Align::Min), |ui| {
                ui.set_min_width(text_width);
                details(ui, view, chain);
            });

            if let (Some((texture, _)), Some(size)) = (illustration, image_size) {
                let image = egui::Image::from_texture((texture, size)).corner_radius(CornerRadius::same(theme::RADIUS));
                ui.add(image).on_hover_ui(|ui| {
                    ui.add(egui::Image::from_texture((texture, size * 2.5)));
                });
            }
        });
    });
}

fn placeholder(ui: &mut Ui, loading: bool) {
    if loading {
        ui.horizontal(|ui| {
            ui.set_height(theme::ICON_SIZE);
            ui.spinner();
            ui.label(widgets::muted("Loading evolution chain…"));
        });
    } else {
        ui.horizontal(|ui| {
            ui.set_height(theme::ICON_SIZE);
            ui.label(RichText::new("No selection").font(theme::bold(theme::TITLE_SIZE)).color(color::TEXT_MUTED));
        });
        ui.label(widgets::muted("Pick a summon from the list to load its evolution chain and skill trees."));
    }
}

fn details(ui: &mut Ui, view: &mut ViewContext<'_>, chain: &EvolutionChain) {
    let summon = &chain.stage3;
    ui.spacing_mut().item_spacing.y = space::XS;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = space::MD;
        let icon = view.textures.icon(summon.face_file_name.as_deref());
        widgets::game_icon(ui, icon, theme::ICON_SIZE);
        ui.label(RichText::new(&summon.summon_name).font(theme::bold(theme::TITLE_SIZE)));
        if view.state.chain_loading {
            ui.spinner();
        }
    });

    ui.add_space(space::XS);
    match summon.feature_text.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
        Some(feature) => ui.add(egui::Label::new(feature).truncate()),
        None => ui.label(widgets::muted("No description.")),
    };
    ui.label(widgets::caption(format!(
        "ID {}   ·   {}  {} ({})",
        summon.summon_id,
        icons::CARD,
        summon.card_name.as_deref().unwrap_or("Unknown card"),
        summon.card_id
    )));

    ui.add_space(space::SM);
    evolution_chain(ui, view, chain);
}

/// The chain as a row of icons joined by arrows, with each summon's ID
/// centred under its icon. Icons and arrows share one centre line.
fn evolution_chain(ui: &mut Ui, view: &mut ViewContext<'_>, chain: &EvolutionChain) {
    let stages: Vec<u8> = (1..=STAGE_COUNT as u8).filter(|stage| chain.stage(*stage).is_some()).collect();
    let width = theme::ICON_SIZE * stages.len() as f32 + ARROW_WIDTH * stages.len().saturating_sub(1) as f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, theme::ICON_SIZE + CHAIN_LABEL_HEIGHT), Sense::hover());
    let icon_center_y = rect.top() + theme::ICON_SIZE / 2.0;

    for (index, stage) in stages.iter().copied().enumerate() {
        let Some(summon) = chain.stage(stage) else {
            continue;
        };
        let left = rect.left() + (theme::ICON_SIZE + ARROW_WIDTH) * index as f32;
        let icon_rect = egui::Rect::from_min_size(egui::pos2(left, rect.top()), Vec2::splat(theme::ICON_SIZE));
        let active = view.state.active_stage == stage;

        if index > 0 {
            ui.painter().text(
                egui::pos2(left - ARROW_WIDTH / 2.0, icon_center_y),
                Align2::CENTER_CENTER,
                icons::ARROW_RIGHT,
                egui::FontId::proportional(theme::GLYPH_SIZE),
                color::TEXT_FAINT,
            );
        }

        let texture = view.textures.icon(summon.face_file_name.as_deref());
        widgets::paint_game_icon(ui, texture, icon_rect, egui::Color32::WHITE);
        if active {
            ui.painter().rect_stroke(
                icon_rect.expand(2.0),
                CornerRadius::same(theme::RADIUS),
                Stroke::new(2.0, color::BORDER_FOCUS),
                StrokeKind::Outside,
            );
        }
        ui.painter().text(
            egui::pos2(icon_rect.center().x, rect.bottom()),
            Align2::CENTER_BOTTOM,
            summon.summon_id.to_string(),
            theme::caption(),
            if active { color::TEXT } else { color::TEXT_MUTED },
        );

        let response = ui
            .interact(icon_rect, ui.id().with(("chain-stage", stage)), Sense::click())
            .on_hover_text(format!("Evolution {stage}: {}", summon.summon_name))
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            view.actions.push(Action::SetStage(stage));
        }
    }
}
