use std::sync::Arc;

use egui::epaint::{CornerRadius, Margin, Shadow, Stroke};
use egui::style::HandleShape;
use egui::{Color32, CursorIcon, FontId, Style, TextStyle, Vec2};

use super::color::ThemeColor;
use super::StudioTheme;

impl StudioTheme {
    pub fn styles(
        &self,
        builtin_dark: &Arc<Style>,
        builtin_light: &Arc<Style>,
    ) -> Result<(Style, Style), String> {
        let mut normalized = self.clone();
        normalized.correct_contrast()?;
        let colors = ResolvedColors::parse(&normalized)?;

        Ok((
            normalized.style_from_base((**builtin_dark).clone(), colors),
            normalized.style_from_base((**builtin_light).clone(), colors),
        ))
    }

    fn style_from_base(&self, mut style: Style, colors: ResolvedColors) -> Style {
        self.apply_spacing(&mut style);
        self.apply_typography(&mut style);
        self.apply_visuals(&mut style, colors);
        style
    }

    fn apply_spacing(&self, style: &mut Style) {
        let ui = &self.ui;
        style.spacing.item_spacing = Vec2::new(
            ui.item_spacing_x.clamp(0.0, 32.0),
            ui.item_spacing_y.clamp(0.0, 32.0),
        );
        style.spacing.window_margin = Margin::same(metric_i8(ui.window_margin));
        style.spacing.menu_margin = Margin::same(metric_i8(ui.menu_margin));
        style.spacing.button_padding = Vec2::new(
            ui.button_padding_x.clamp(0.0, 32.0),
            ui.button_padding_y.clamp(0.0, 24.0),
        );
        style.spacing.interact_size.y = ui.interact_height.clamp(20.0, 64.0);
        style.spacing.slider_width = ui.slider_width.clamp(80.0, 400.0);
        style.spacing.scroll.bar_width = ui.scrollbar_width.clamp(4.0, 24.0);
        style.spacing.scroll.handle_min_length = ui.scrollbar_min_handle.clamp(8.0, 96.0);
        style.animation_time = (ui.animation_ms / 1000.0).clamp(0.0, 2.0);
    }

    fn apply_typography(&self, style: &mut Style) {
        let typography = &self.typography;
        style.text_styles.insert(
            TextStyle::Body,
            FontId::proportional(font_size(typography.body_size)),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::proportional(font_size(typography.button_size)),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::proportional(font_size(typography.small_size)),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::proportional(font_size(typography.heading_size)),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::monospace(font_size(typography.monospace_size)),
        );
    }

    fn apply_visuals(&self, style: &mut Style, colors: ResolvedColors) {
        let visuals = &mut style.visuals;
        let widget_radius = CornerRadius::same(metric_u8(self.ui.widget_radius));
        let border_width = self.ui.border_width.clamp(0.5, 4.0);
        let border = Stroke::new(border_width, colors.border);
        let focus_border = Stroke::new(border_width.max(1.0), colors.border_focus);

        visuals.dark_mode = colors.background.is_dark();
        visuals.override_text_color = None;
        visuals.weak_text_color = Some(colors.muted_text.to_egui());
        visuals.weak_text_alpha = 1.0;

        visuals.widgets.noninteractive.bg_fill = colors.panel.to_egui();
        visuals.widgets.noninteractive.weak_bg_fill = colors.panel.to_egui();
        visuals.widgets.noninteractive.bg_stroke = border;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.text);
        visuals.widgets.noninteractive.corner_radius = widget_radius;

        visuals.widgets.inactive.bg_fill = colors.input.to_egui();
        visuals.widgets.inactive.weak_bg_fill = colors.surface.to_egui();
        visuals.widgets.inactive.bg_stroke = border;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.text);
        visuals.widgets.inactive.corner_radius = widget_radius;

        visuals.widgets.hovered.bg_fill = colors.surface_alt.to_egui();
        visuals.widgets.hovered.weak_bg_fill = colors.surface_alt.to_egui();
        visuals.widgets.hovered.bg_stroke = focus_border;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, colors.accent_hover);
        visuals.widgets.hovered.corner_radius = widget_radius;
        visuals.widgets.hovered.expansion = 1.0;

        visuals.widgets.active.bg_fill = colors.selection.to_egui();
        visuals.widgets.active.weak_bg_fill = colors.selection.to_egui();
        visuals.widgets.active.bg_stroke = focus_border;
        visuals.widgets.active.fg_stroke = Stroke::new(1.5, colors.selection_text);
        visuals.widgets.active.corner_radius = widget_radius;

        visuals.widgets.open.bg_fill = colors.surface_alt.to_egui();
        visuals.widgets.open.weak_bg_fill = colors.surface_alt.to_egui();
        visuals.widgets.open.bg_stroke = focus_border;
        visuals.widgets.open.fg_stroke = Stroke::new(1.5, colors.text);
        visuals.widgets.open.corner_radius = widget_radius;

        visuals.selection.bg_fill = colors.selection.to_egui();
        visuals.selection.stroke = Stroke::new(1.0, colors.selection_text);
        visuals.hyperlink_color = colors.hyperlink.to_egui();
        visuals.faint_bg_color = colors.surface.to_egui();
        visuals.extreme_bg_color = colors.input.to_egui();
        visuals.text_edit_bg_color = Some(colors.input.to_egui());
        visuals.code_bg_color = colors.code_background.to_egui();
        visuals.warn_fg_color = colors.warning.to_egui();
        visuals.error_fg_color = colors.error.to_egui();
        visuals.window_corner_radius = CornerRadius::same(metric_u8(self.ui.window_radius));
        visuals.window_fill = colors.background.to_egui();
        visuals.window_stroke = border;
        visuals.menu_corner_radius = CornerRadius::same(metric_u8(self.ui.menu_radius));
        visuals.panel_fill = colors.panel.to_egui();
        visuals.window_shadow = self.shadow(colors.shadow);
        visuals.popup_shadow = self.shadow(colors.shadow);
        visuals.text_cursor.stroke = Stroke::new(2.0, colors.accent);
        visuals.slider_trailing_fill = true;
        visuals.handle_shape = HandleShape::Rect { aspect_ratio: 0.75 };
        visuals.interact_cursor = Some(CursorIcon::PointingHand);
        visuals.disabled_alpha = self.accessibility.disabled_opacity.clamp(0.5, 1.0);
    }

    fn shadow(&self, color: Color32) -> Shadow {
        Shadow {
            offset: [
                metric_i8(self.ui.shadow_offset_x),
                metric_i8(self.ui.shadow_offset_y),
            ],
            blur: metric_u8(self.ui.shadow_blur),
            spread: 0,
            color,
        }
    }
}

#[derive(Clone, Copy)]
struct ResolvedColors {
    background: ThemeColor,
    panel: ThemeColor,
    surface: ThemeColor,
    surface_alt: ThemeColor,
    input: ThemeColor,
    text: Color32,
    muted_text: ThemeColor,
    accent: Color32,
    accent_hover: Color32,
    selection: ThemeColor,
    selection_text: Color32,
    border: Color32,
    border_focus: Color32,
    warning: ThemeColor,
    error: ThemeColor,
    hyperlink: ThemeColor,
    code_background: ThemeColor,
    shadow: Color32,
}

impl ResolvedColors {
    fn parse(theme: &StudioTheme) -> Result<Self, String> {
        let colors = &theme.colors;
        Ok(Self {
            background: ThemeColor::parse(&colors.background)?,
            panel: ThemeColor::parse(&colors.panel)?,
            surface: ThemeColor::parse(&colors.surface)?,
            surface_alt: ThemeColor::parse(&colors.surface_alt)?,
            input: ThemeColor::parse(&colors.input)?,
            text: ThemeColor::parse(&colors.text)?.to_egui(),
            muted_text: ThemeColor::parse(&colors.muted_text)?,
            accent: ThemeColor::parse(&colors.accent)?.to_egui(),
            accent_hover: ThemeColor::parse(&colors.accent_hover)?.to_egui(),
            selection: ThemeColor::parse(&colors.selection)?,
            selection_text: ThemeColor::parse(&colors.selection_text)?.to_egui(),
            border: ThemeColor::parse(&colors.border)?.to_egui(),
            border_focus: ThemeColor::parse(&colors.border_focus)?.to_egui(),
            warning: ThemeColor::parse(&colors.warning)?,
            error: ThemeColor::parse(&colors.error)?,
            hyperlink: ThemeColor::parse(&colors.hyperlink)?,
            code_background: ThemeColor::parse(&colors.code_background)?,
            shadow: ThemeColor::parse(&colors.shadow)?.to_egui(),
        })
    }
}

fn metric_i8(value: f32) -> i8 {
    value.round().clamp(i8::MIN as f32, i8::MAX as f32) as i8
}

fn metric_u8(value: f32) -> u8 {
    value.round().clamp(0.0, u8::MAX as f32) as u8
}

fn font_size(value: f32) -> f32 {
    value.clamp(8.0, 40.0)
}
