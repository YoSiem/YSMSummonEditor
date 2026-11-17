//! Header card: the selected summon, its evolution chain and illustration.

use eframe::egui::{self, Align, CornerRadius, Layout, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use ysm_core::model::{EvolutionChain, STAGE_COUNT};

use super::ViewContext;
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets;

const CARD_HEIGHT: f32 = 148.0;

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    theme::card().show(ui, |ui| {
        ui.set_min_height(CARD_HEIGHT);
        ui.set_min_width(ui.available_width());

        let Some(chain) = view.state.session.chain() else {
            if view.state.chain_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(widgets::muted("Loading evolution chain…"));
                });
            } else {
                ui.label(RichText::new("No selection").font(theme::bold(24.0)).color(color::TEXT_MUTED));
                ui.label(widgets::muted("Pick a summon from the list to load its evolution chain and skill trees."));
            }
            return;
        };

        let final_stage = &chain.stage3;
        let illustration = view
            .textures
            .illustration(final_stage.illust_file_name.as_deref())
            .map(|texture| (texture.id(), texture.aspect_ratio()));
        let image_size = illustration.map(|(_, aspect)| Vec2::new(CARD_HEIGHT * aspect, CARD_HEIGHT));
        let image_width = image_size.map_or(0.0, |size| size.x + 12.0);

        ui.horizontal(|ui| {
            let text_width = (ui.available_width() - image_width).max(200.0);
            ui.allocate_ui_with_layout(Vec2::new(text_width, CARD_HEIGHT), Layout::top_down(Align::Min), |ui| {
                ui.set_min_width(text_width);
                details(ui, view, chain);
            });

            if let (Some((texture, _)), Some(size)) = (illustration, image_size) {
                let image = egui::Image::from_texture((texture, size)).corner_radius(CornerRadius::same(6));
                ui.add(image).on_hover_ui(|ui| {
                    ui.add(egui::Image::from_texture((texture, size * 2.6)));
                });
            }
        });
    });
}

fn details(ui: &mut Ui, view: &mut ViewContext<'_>, chain: &EvolutionChain) {
    let summon = &chain.stage3;

    ui.horizontal(|ui| {
        let icon = view.textures.icon(summon.face_file_name.as_deref());
        widgets::game_icon(ui, icon, theme::ICON_SIZE);
        ui.label(RichText::new(&summon.summon_name).font(theme::bold(25.0)));
        if view.state.chain_loading {
            ui.spinner();
        }
    });

    if let Some(feature) = summon.feature_text.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
        ui.add(egui::Label::new(feature).wrap());
    }

    ui.label(widgets::muted(format!(
        "ID {}   {}  {} ({})",
        summon.summon_id,
        icons::CARD,
        summon.card_name.as_deref().unwrap_or("Unknown card"),
        summon.card_id
    )));

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let mut first = true;
        for stage in 1..=STAGE_COUNT as u8 {
            let Some(stage_summon) = chain.stage(stage) else {
                continue;
            };
            if !std::mem::take(&mut first) {
                ui.label(RichText::new(icons::ARROW_RIGHT).size(16.0).color(color::TEXT_MUTED));
            }

            let active = view.state.active_stage == stage;
            let icon = view.textures.icon(stage_summon.face_file_name.as_deref());
            let response = ui
                .vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let icon_response = widgets::game_icon(ui, icon, theme::ICON_SIZE);
                    if active {
                        ui.painter().rect_stroke(
                            icon_response.rect.expand(2.0),
                            CornerRadius::same(5),
                            Stroke::new(2.0, color::ACCENT_BRIGHT),
                            StrokeKind::Outside,
                        );
                    }
                    let tint = if active { color::TEXT } else { color::TEXT_MUTED };
                    ui.label(RichText::new(stage_summon.summon_id.to_string()).small().color(tint));
                })
                .response
                .interact(Sense::click())
                .on_hover_text(format!("Evolution {stage}: {}", stage_summon.summon_name))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                view.actions.push(Action::SetStage(stage));
            }
        }
    });
}
