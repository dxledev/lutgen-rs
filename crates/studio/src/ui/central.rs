use crate::ui::scene::Scene;
use crate::App;

impl App {
    fn show_image(&mut self, mut size: egui::Vec2, ui: &mut egui::Ui) {
        let corner_radius = ui.visuals().widgets.noninteractive.corner_radius;
        if self.state.edited_texture.is_none() && self.state.image_texture.is_none() {
            // no image loaded
            if self.inline_layout {
                size.y /= 4.;
            }
            let (rect, res) = ui.allocate_exact_size(size, egui::Sense::click());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Load image ...",
                egui::FontId::default(),
                ui.visuals().weak_text_color(),
            );
            if res.clicked() {
                self.open_picker.trigger(None);
            }
        } else {
            let rect = if !self.state.show_original
                && let Some(texture) = &self.state.edited_texture
            {
                // display edited image
                let res = ui.add(
                    egui::Image::from_texture(texture)
                        .texture_options(egui::TextureOptions::NEAREST)
                        .max_size(size)
                        .corner_radius(corner_radius)
                        .sense(egui::Sense::click()),
                );
                if res.clicked() {
                    self.state.show_original = true;
                }
                res.rect
            } else if let Some(texture) = &self.state.image_texture {
                // display original image
                let res = ui.add(
                    egui::Image::from_texture(texture)
                        .texture_options(egui::TextureOptions::NEAREST)
                        .max_size(size)
                        .corner_radius(corner_radius)
                        .sense(egui::Sense::click()),
                );
                if res.clicked() {
                    if self.state.edited_texture.is_none() {
                        // apply if there's no edited texture to show
                        self.apply();
                    }
                    self.state.show_original = false;
                }

                // draw "original" indicator
                let painter = ui.painter();
                let pos = res.rect.min + egui::Vec2::splat(8.);
                let text_color = ui.visuals().text_color();
                let galley = painter.layout_no_wrap(
                    "Original".to_string(),
                    egui::TextStyle::Body.resolve(ui.style()),
                    text_color,
                );
                let text_rect = egui::Rect::from_min_size(pos, galley.size());
                let padding = egui::Vec2::splat(6.0);
                let bg_rect = text_rect.expand2(padding);
                let background = ui.visuals().window_fill();
                let background = egui::Color32::from_rgba_unmultiplied(
                    background.r(),
                    background.g(),
                    background.b(),
                    220,
                );
                painter.rect_filled(bg_rect, corner_radius, background);
                painter.galley(pos, galley, text_color);

                res.rect
            } else {
                unreachable!();
            };

            // paint border
            ui.painter().rect_stroke(
                rect,
                corner_radius,
                ui.visuals().window_stroke(),
                egui::StrokeKind::Middle,
            );

            // show spinner if processing
            if self.state.processing {
                let panel = ui.visuals().panel_fill;
                let overlay =
                    egui::Color32::from_rgba_unmultiplied(panel.r(), panel.g(), panel.b(), 96);
                ui.painter().rect_filled(rect, corner_radius, overlay);
                egui::Spinner::new().paint_at(
                    ui,
                    egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(30.)),
                );
            }
        }
    }

    /// Main app panel
    pub fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let size = ui.available_size();
            if !self.inline_layout {
                let mut rect = self.scene_rect;
                Scene::new()
                    .zoom_range(0.95..=f32::INFINITY)
                    .constrain_panning(true)
                    .show(ui, &mut rect, |ui| {
                        self.show_image(size, ui);
                    });
                self.scene_rect = rect;
            } else {
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    self.show_image(size, ui);
                    self.show_sidebar_inner(ui);
                });
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
