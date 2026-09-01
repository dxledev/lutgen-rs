mod color;
mod source;
mod style;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use egui::{Color32, Style, Theme, ThemePreference};
use serde::{Deserialize, Serialize};

use color::ThemeColor;
use source::{SourceSnapshot, ThemePaths, ThemeSource, ThemeValue};

const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StudioTheme {
    pub metadata: ThemeMetadata,
    #[serde(default)]
    pub colors: ThemeColors,
    #[serde(default)]
    pub ui: UiMetrics,
    #[serde(default)]
    pub typography: Typography,
    #[serde(default)]
    pub accessibility: Accessibility,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub active_theme: String,
    pub source_fingerprint: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub background: String,
    pub panel: String,
    pub surface: String,
    pub surface_alt: String,
    pub input: String,
    pub text: String,
    pub muted_text: String,
    pub accent: String,
    pub accent_hover: String,
    pub selection: String,
    pub selection_text: String,
    pub border: String,
    pub border_focus: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub hyperlink: String,
    pub code_background: String,
    pub overlay: String,
    pub shadow: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: "#191724".into(),
            panel: "#1e1c28".into(),
            surface: "#302d3e".into(),
            surface_alt: "#373540".into(),
            input: "#26233a".into(),
            text: "#e0def4".into(),
            muted_text: "#908caa".into(),
            accent: "#9ccfd8".into(),
            accent_hover: "#a8d5dd".into(),
            selection: "#9ccfd8".into(),
            selection_text: "#111111".into(),
            border: "#7da3ad".into(),
            border_focus: "#9ccfd8".into(),
            success: "#56949f".into(),
            warning: "#f6c177".into(),
            error: "#eb6f92".into(),
            hyperlink: "#9ccfd8".into(),
            code_background: "#26233a".into(),
            overlay: "#191724a6".into(),
            shadow: "#00000078".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiMetrics {
    pub window_radius: f32,
    pub menu_radius: f32,
    pub widget_radius: f32,
    pub border_width: f32,
    pub window_margin: f32,
    pub menu_margin: f32,
    pub item_spacing_x: f32,
    pub item_spacing_y: f32,
    pub button_padding_x: f32,
    pub button_padding_y: f32,
    pub interact_height: f32,
    pub slider_width: f32,
    pub scrollbar_width: f32,
    pub scrollbar_min_handle: f32,
    pub animation_ms: f32,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_blur: f32,
}

impl Default for UiMetrics {
    fn default() -> Self {
        Self {
            window_radius: 16.0,
            menu_radius: 10.0,
            widget_radius: 8.0,
            border_width: 1.0,
            window_margin: 12.0,
            menu_margin: 8.0,
            item_spacing_x: 8.0,
            item_spacing_y: 8.0,
            button_padding_x: 12.0,
            button_padding_y: 6.0,
            interact_height: 34.0,
            slider_width: 160.0,
            scrollbar_width: 8.0,
            scrollbar_min_handle: 28.0,
            animation_ms: 220.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 4.0,
            shadow_blur: 16.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Typography {
    pub body_size: f32,
    pub button_size: f32,
    pub small_size: f32,
    pub heading_size: f32,
    pub monospace_size: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            body_size: 13.0,
            button_size: 13.0,
            small_size: 11.0,
            heading_size: 17.0,
            monospace_size: 13.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Accessibility {
    pub minimum_text_contrast: f32,
    pub minimum_muted_text_contrast: f32,
    pub minimum_control_contrast: f32,
    pub disabled_opacity: f32,
}

impl Default for Accessibility {
    fn default() -> Self {
        Self {
            minimum_text_contrast: 7.0,
            minimum_muted_text_contrast: 4.5,
            minimum_control_contrast: 4.5,
            disabled_opacity: 0.65,
        }
    }
}

pub struct ThemeManager {
    paths: ThemePaths,
    builtin_dark: Arc<Style>,
    builtin_light: Arc<Style>,
    current: Option<StudioTheme>,
    custom_applied: bool,
    last_preference: Option<ThemePreference>,
    next_refresh: Instant,
    watch_cache: ThemeWatchCache,
}

impl ThemeManager {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut manager = Self {
            paths: ThemePaths::from_env(),
            builtin_dark: ctx.style_of(Theme::Dark),
            builtin_light: ctx.style_of(Theme::Light),
            current: None,
            custom_applied: false,
            last_preference: None,
            next_refresh: Instant::now(),
            watch_cache: ThemeWatchCache::default(),
        };
        manager.update(ctx);
        manager
    }

    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let preference = ctx.options(|options| options.theme_preference);
        let preference_changed = self.last_preference != Some(preference);
        self.last_preference = Some(preference);

        if preference != ThemePreference::System {
            if self.custom_applied {
                self.restore_builtin(ctx);
                return true;
            }
            return false;
        }

        ctx.request_repaint_after(REFRESH_INTERVAL);
        if !preference_changed && Instant::now() < self.next_refresh {
            return false;
        }
        self.next_refresh = Instant::now() + REFRESH_INTERVAL;

        if !preference_changed && self.custom_applied && !self.watch_cache.changed() {
            return false;
        }

        match load_or_generate(&self.paths) {
            Ok(loaded) => {
                let theme_changed = self.current.as_ref() != Some(&loaded.theme);
                let should_apply = theme_changed || !self.custom_applied;
                if loaded.regenerated {
                    log::info!(
                        "Regenerated system theme at {}",
                        self.paths.output.display()
                    );
                }

                let applied = if should_apply {
                    match loaded.theme.styles(&self.builtin_dark, &self.builtin_light) {
                        Ok((dark, light)) => {
                            ctx.set_style_of(Theme::Dark, dark);
                            ctx.set_style_of(Theme::Light, light);
                            self.custom_applied = true;
                            true
                        },
                        Err(error) => {
                            log::warn!("Failed to apply generated system theme: {error}");
                            false
                        },
                    }
                } else {
                    false
                };

                self.watch_cache
                    .refresh(theme_watch_paths(&self.paths, &loaded.theme));
                self.current = Some(loaded.theme);
                applied
            },
            Err(error) => {
                log::warn!("Failed to refresh generated system theme: {error}");
                false
            },
        }
    }

    pub fn dialog_overlay(&self) -> Color32 {
        if !self.custom_applied {
            return Color32::from_black_alpha(120);
        }
        self.current
            .as_ref()
            .and_then(|theme| ThemeColor::parse(&theme.colors.overlay).ok())
            .map(ThemeColor::to_egui)
            .unwrap_or_else(|| Color32::from_black_alpha(120))
    }

    pub fn active_theme_name(&self) -> Option<&str> {
        self.current
            .as_ref()
            .map(|theme| theme.metadata.active_theme.as_str())
    }

    pub fn theme_path(&self) -> &Path {
        &self.paths.output
    }

    fn restore_builtin(&mut self, ctx: &egui::Context) {
        ctx.set_style_of(Theme::Dark, self.builtin_dark.clone());
        ctx.set_style_of(Theme::Light, self.builtin_light.clone());
        self.custom_applied = false;
    }
}

pub fn generate_system_theme() -> Result<PathBuf, String> {
    let paths = ThemePaths::from_env();
    let snapshot = SourceSnapshot::collect(&paths)?;
    let theme = StudioTheme::generate(&snapshot)?;
    theme.styles(&Arc::new(Style::default()), &Arc::new(Style::default()))?;
    write_theme(&paths.output, &theme)?;
    Ok(paths.output)
}

struct LoadedTheme {
    theme: StudioTheme,
    regenerated: bool,
}

fn load_or_generate(paths: &ThemePaths) -> Result<LoadedTheme, String> {
    let snapshot = SourceSnapshot::collect(paths)?;
    if let Ok(theme) = read_theme(&paths.output)
        && theme.metadata.source_fingerprint == snapshot.fingerprint
        && theme
            .styles(&Arc::new(Style::default()), &Arc::new(Style::default()))
            .is_ok()
    {
        return Ok(LoadedTheme {
            theme,
            regenerated: false,
        });
    }

    let theme = StudioTheme::generate(&snapshot)?;
    write_theme(&paths.output, &theme)?;
    Ok(LoadedTheme {
        theme,
        regenerated: true,
    })
}

#[derive(Default)]
struct ThemeWatchCache {
    files: HashMap<PathBuf, FileStamp>,
}

impl ThemeWatchCache {
    fn changed(&self) -> bool {
        self.files.is_empty()
            || self
                .files
                .iter()
                .any(|(path, stamp)| *stamp != FileStamp::read(path))
    }

    fn refresh(&mut self, paths: Vec<PathBuf>) {
        self.files = paths
            .into_iter()
            .map(|path| {
                let stamp = FileStamp::read(&path);
                (path, stamp)
            })
            .collect();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileStamp {
    resolved_path: Option<PathBuf>,
    link_modified: Option<SystemTime>,
    target_modified: Option<SystemTime>,
    target_length: Option<u64>,
}

impl FileStamp {
    fn read(path: &Path) -> Self {
        let link_metadata = fs::symlink_metadata(path).ok();
        let target_metadata = fs::metadata(path).ok();
        Self {
            resolved_path: fs::canonicalize(path).ok(),
            link_modified: link_metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok()),
            target_modified: target_metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok()),
            target_length: target_metadata.map(|metadata| metadata.len()),
        }
    }
}

fn theme_watch_paths(paths: &ThemePaths, theme: &StudioTheme) -> Vec<PathBuf> {
    let mut watched = paths.watch_paths();
    watched.extend(theme.metadata.sources.iter().map(PathBuf::from));
    watched.sort();
    watched.dedup();
    watched
}

fn read_theme(path: &Path) -> Result<StudioTheme, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_theme(path: &Path, theme: &StudioTheme) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let contents = toml::to_string_pretty(theme)
        .map_err(|error| format!("failed to serialize system theme: {error}"))?;
    let contents = format!(
        "# Generated from Chromack, Dashboard, and WardNC.\n# Edit values freely; source theme changes regenerate this file.\n\n{contents}"
    );
    fs::write(path, contents)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

impl StudioTheme {
    fn generate(snapshot: &SourceSnapshot) -> Result<Self, String> {
        use ThemeSource::{Chromack, DashboardStyle, DashboardTheme, WardNc};

        let defaults = ThemeColors::default();
        let mut colors = ThemeColors {
            background: color_hex(
                snapshot,
                &defaults.background,
                &[
                    ThemeValue::new(DashboardTheme, "bg"),
                    ThemeValue::new(Chromack, "color-surface"),
                    ThemeValue::new(WardNc, "list-bg"),
                ],
            ),
            panel: color_hex(
                snapshot,
                &defaults.panel,
                &[
                    ThemeValue::new(WardNc, "panel-bg"),
                    ThemeValue::new(Chromack, "panel-bg"),
                    ThemeValue::new(DashboardTheme, "bg"),
                ],
            ),
            surface: color_hex(
                snapshot,
                &defaults.surface,
                &[
                    ThemeValue::new(Chromack, "content-bg"),
                    ThemeValue::new(WardNc, "header-bg"),
                ],
            ),
            surface_alt: color_hex(
                snapshot,
                &defaults.surface_alt,
                &[
                    ThemeValue::new(WardNc, "card-bg"),
                    ThemeValue::new(Chromack, "color-surface-alt"),
                ],
            ),
            input: color_hex(
                snapshot,
                &defaults.input,
                &[
                    ThemeValue::new(Chromack, "input-bg"),
                    ThemeValue::new(WardNc, "action-bg"),
                ],
            ),
            text: color_hex(
                snapshot,
                &defaults.text,
                &[
                    ThemeValue::new(DashboardTheme, "fg"),
                    ThemeValue::new(Chromack, "color-text"),
                    ThemeValue::new(WardNc, "summary-color"),
                ],
            ),
            muted_text: color_hex(
                snapshot,
                &defaults.muted_text,
                &[
                    ThemeValue::new(DashboardTheme, "muted"),
                    ThemeValue::new(Chromack, "color-muted"),
                    ThemeValue::new(WardNc, "body-color"),
                ],
            ),
            accent: color_hex(
                snapshot,
                &defaults.accent,
                &[
                    ThemeValue::new(DashboardTheme, "accent"),
                    ThemeValue::new(Chromack, "color-primary"),
                    ThemeValue::new(WardNc, "app-name-color"),
                ],
            ),
            accent_hover: color_hex(
                snapshot,
                &defaults.accent_hover,
                &[
                    ThemeValue::new(WardNc, "clear-hover-bg"),
                    ThemeValue::new(Chromack, "color-secondary"),
                    ThemeValue::new(DashboardTheme, "secondary"),
                ],
            ),
            selection: color_hex(
                snapshot,
                &defaults.selection,
                &[
                    ThemeValue::new(Chromack, "color-primary"),
                    ThemeValue::new(WardNc, "clear-bg"),
                ],
            ),
            selection_text: color_hex(
                snapshot,
                &defaults.selection_text,
                &[
                    ThemeValue::new(Chromack, "button-primary-fg"),
                    ThemeValue::new(WardNc, "clear-color"),
                ],
            ),
            border: color_hex(
                snapshot,
                &defaults.border,
                &[
                    ThemeValue::new(WardNc, "panel-border"),
                    ThemeValue::new(Chromack, "color-border"),
                ],
            ),
            border_focus: color_hex(
                snapshot,
                &defaults.border_focus,
                &[
                    ThemeValue::new(DashboardTheme, "border"),
                    ThemeValue::new(Chromack, "color-secondary"),
                ],
            ),
            success: color_hex(
                snapshot,
                &defaults.success,
                &[
                    ThemeValue::new(Chromack, "color-success"),
                    ThemeValue::new(DashboardTheme, "green"),
                ],
            ),
            warning: color_hex(
                snapshot,
                &defaults.warning,
                &[ThemeValue::new(Chromack, "color-warning")],
            ),
            error: color_hex(
                snapshot,
                &defaults.error,
                &[ThemeValue::new(Chromack, "color-danger")],
            ),
            hyperlink: color_hex(
                snapshot,
                &defaults.hyperlink,
                &[
                    ThemeValue::new(DashboardTheme, "accent"),
                    ThemeValue::new(WardNc, "app-name-color"),
                ],
            ),
            code_background: color_hex(
                snapshot,
                &defaults.code_background,
                &[
                    ThemeValue::new(Chromack, "input-bg"),
                    ThemeValue::new(WardNc, "action-bg"),
                ],
            ),
            overlay: color_hex(
                snapshot,
                &defaults.overlay,
                &[ThemeValue::new(Chromack, "color-panel-backdrop")],
            ),
            shadow: defaults.shadow,
        };
        apply_generated_alpha(&mut colors)?;

        let mut ui = UiMetrics::default();
        ui.window_radius = number(
            snapshot,
            ui.window_radius,
            &[
                ThemeValue::new(DashboardStyle, "radiusTopLeft"),
                ThemeValue::new(Chromack, "panel-radius"),
                ThemeValue::new(WardNc, "panel-radius"),
            ],
        );
        ui.menu_radius = number(
            snapshot,
            ui.menu_radius,
            &[
                ThemeValue::new(WardNc, "header-radius"),
                ThemeValue::new(Chromack, "header-radius"),
            ],
        );
        ui.widget_radius = number(
            snapshot,
            ui.widget_radius,
            &[
                ThemeValue::new(Chromack, "input-radius"),
                ThemeValue::new(WardNc, "action-radius"),
            ],
        );
        ui.border_width = number(
            snapshot,
            ui.border_width,
            &[
                ThemeValue::new(Chromack, "panel-border-width"),
                ThemeValue::new(WardNc, "panel-border-width"),
                ThemeValue::new(DashboardStyle, "borderWidth"),
            ],
        );
        ui.window_margin = number(
            snapshot,
            ui.window_margin,
            &[
                ThemeValue::new(Chromack, "panel-padding"),
                ThemeValue::new(WardNc, "panel-padding-x"),
            ],
        );
        ui.item_spacing_x = number(
            snapshot,
            ui.item_spacing_x,
            &[
                ThemeValue::new(WardNc, "header-gap"),
                ThemeValue::new(Chromack, "section-gap"),
            ],
        );
        ui.item_spacing_y = number(
            snapshot,
            ui.item_spacing_y,
            &[
                ThemeValue::new(WardNc, "text-gap"),
                ThemeValue::new(Chromack, "section-gap"),
            ],
        );
        ui.button_padding_x = number(
            snapshot,
            ui.button_padding_x,
            &[
                ThemeValue::new(Chromack, "button-padding-x"),
                ThemeValue::new(WardNc, "header-padding-x"),
            ],
        );
        ui.interact_height = number(
            snapshot,
            ui.interact_height,
            &[
                ThemeValue::new(Chromack, "button-height"),
                ThemeValue::new(WardNc, "header-height"),
            ],
        );
        ui.scrollbar_width = number(
            snapshot,
            ui.scrollbar_width,
            &[ThemeValue::new(WardNc, "scrollbar-width")],
        );
        ui.scrollbar_min_handle = number(
            snapshot,
            ui.scrollbar_min_handle,
            &[ThemeValue::new(WardNc, "scrollbar-thumb-min-height")],
        );
        ui.animation_ms = number(
            snapshot,
            ui.animation_ms,
            &[ThemeValue::new(DashboardStyle, "tabSlideDuration")],
        );

        let mut typography = Typography::default();
        typography.body_size = number(
            snapshot,
            typography.body_size,
            &[
                ThemeValue::new(WardNc, "body-size"),
                ThemeValue::new(Chromack, "section-title-size"),
            ],
        );
        typography.button_size = number(
            snapshot,
            typography.button_size,
            &[
                ThemeValue::new(WardNc, "action-size"),
                ThemeValue::new(Chromack, "section-title-size"),
            ],
        );
        typography.small_size = number(
            snapshot,
            typography.small_size,
            &[
                ThemeValue::new(WardNc, "timestamp-size"),
                ThemeValue::new(Chromack, "subtitle-size"),
            ],
        );
        typography.heading_size = number(
            snapshot,
            typography.heading_size,
            &[
                ThemeValue::new(Chromack, "title-size"),
                ThemeValue::new(DashboardStyle, "fontSize"),
            ],
        );
        typography.monospace_size = typography.body_size;

        let mut theme = Self {
            metadata: ThemeMetadata {
                active_theme: snapshot.active_theme.clone(),
                source_fingerprint: snapshot.fingerprint.clone(),
                sources: snapshot.source_paths.clone(),
            },
            colors,
            ui,
            typography,
            accessibility: Accessibility::default(),
        };
        theme.correct_contrast()?;
        Ok(theme)
    }

    fn correct_contrast(&mut self) -> Result<(), String> {
        let background = ThemeColor::parse(&self.colors.background)?;
        let panel = ThemeColor::parse(&self.colors.panel)?;
        let selection = ThemeColor::parse(&self.colors.selection)?;
        let text_minimum = self.accessibility.minimum_text_contrast.max(4.5);
        let muted_minimum = self.accessibility.minimum_muted_text_contrast.max(4.5);
        let control_minimum = self.accessibility.minimum_control_contrast.max(4.5);

        self.colors.text = ThemeColor::parse(&self.colors.text)?
            .ensure_contrast_against(&[background, panel], text_minimum)
            .to_hex();
        self.colors.muted_text = ThemeColor::parse(&self.colors.muted_text)?
            .ensure_contrast_against(&[background, panel], muted_minimum)
            .to_hex();
        for value in [
            &mut self.colors.accent,
            &mut self.colors.success,
            &mut self.colors.warning,
            &mut self.colors.error,
            &mut self.colors.hyperlink,
        ] {
            *value = ThemeColor::parse(value)?
                .ensure_contrast(background, control_minimum)
                .to_hex();
        }
        let selection_text = ThemeColor::parse(&self.colors.selection_text)?;
        self.colors.selection_text = if selection_text.contrast_ratio(selection) >= control_minimum
        {
            selection_text
        } else {
            ThemeColor::best_text_color(selection)
        }
        .to_hex();
        Ok(())
    }
}

fn color_hex(snapshot: &SourceSnapshot, fallback: &str, candidates: &[ThemeValue<'_>]) -> String {
    snapshot
        .color(candidates)
        .map(ThemeColor::to_hex)
        .unwrap_or_else(|| fallback.to_string())
}

fn number(snapshot: &SourceSnapshot, fallback: f32, candidates: &[ThemeValue<'_>]) -> f32 {
    snapshot
        .number(candidates)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(fallback)
}

fn apply_generated_alpha(colors: &mut ThemeColors) -> Result<(), String> {
    let overlay = ThemeColor::parse(&colors.overlay)?.with_alpha(166);
    colors.overlay = overlay.to_hex();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::color::ThemeColor;
    use super::source::{SourceSnapshot, ThemePaths};
    use super::{load_or_generate, theme_watch_paths, write_theme, StudioTheme, ThemeWatchCache};

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("lutgen-theme-test-{unique}"))
    }

    fn write_sources(root: &std::path::Path, accent: &str) {
        fs::create_dir_all(root.join("chromack")).unwrap();
        fs::create_dir_all(root.join("dashboard")).unwrap();
        fs::create_dir_all(root.join("wardnc")).unwrap();
        fs::create_dir_all(root.join("themes/current")).unwrap();

        fs::write(
            root.join("chromack/config.toml"),
            "[paths]\nstyle_css = \"style.css\"\ncolors_css = \"colors.css\"\nmaterial_css = \"material.css\"\n",
        )
        .unwrap();
        fs::write(
            root.join("chromack/style.css"),
            ":root { --panel-radius: 16px; }",
        )
        .unwrap();
        fs::write(
            root.join("chromack/colors.css"),
            ":root {\n--color-surface: #191724;\n--color-text: #e0def4;\n--color-primary: #9ccfd8;\n--color-warning: #f6c177;\n--color-danger: #eb6f92;\n}",
        )
        .unwrap();
        fs::write(root.join("chromack/material.css"), ":root {}\n").unwrap();
        fs::write(
            root.join("dashboard/theme.qml"),
            format!(
                "property color bg: \"#191724\"\nproperty color fg: \"#e0def4\"\nproperty color muted: \"#908caa\"\nproperty color accent: \"{accent}\""
            ),
        )
        .unwrap();
        fs::write(
            root.join("dashboard/style.qml"),
            "property int radiusTopLeft: 16\nproperty int fontSize: 16",
        )
        .unwrap();
        fs::write(
            root.join("wardnc/config.toml"),
            "[paths]\nstyle_css = \"style.css\"\ncolors_css = \"colors.css\"\n",
        )
        .unwrap();
        fs::write(
            root.join("wardnc/colors.css"),
            ":root { --panel-bg: #1e1c28; }",
        )
        .unwrap();
        fs::write(
            root.join("wardnc/style.css"),
            ":root {\n--card-bg: #373540;\n--panel-border: #7da3ad;\n}\n",
        )
        .unwrap();
    }

    #[test]
    fn combines_active_desktop_theme_sources() {
        let root = temp_root();
        let paths = ThemePaths::test(&root);
        write_sources(&root, "#9ccfd8");

        let snapshot = SourceSnapshot::collect(&paths).unwrap();
        let theme = StudioTheme::generate(&snapshot).unwrap();
        let background = ThemeColor::parse(&theme.colors.background).unwrap();
        let text = ThemeColor::parse(&theme.colors.text).unwrap();

        assert_eq!(theme.colors.background, "#191724");
        assert_eq!(theme.colors.accent, "#9ccfd8");
        assert!(text.contrast_ratio(background) >= 7.0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_local_edits_until_a_source_theme_changes() {
        let root = temp_root();
        let paths = ThemePaths::test(&root);
        write_sources(&root, "#9ccfd8");

        let generated = load_or_generate(&paths).unwrap();
        assert!(generated.regenerated);
        let mut edited = generated.theme.clone();
        edited.typography.body_size = 18.0;
        write_theme(&paths.output, &edited).unwrap();

        let loaded = load_or_generate(&paths).unwrap();
        assert!(!loaded.regenerated);
        assert_eq!(loaded.theme.typography.body_size, 18.0);

        write_sources(&root, "#f6c177");
        let regenerated = load_or_generate(&paths).unwrap();
        assert!(regenerated.regenerated);
        assert_ne!(
            regenerated.theme.metadata.source_fingerprint,
            generated.theme.metadata.source_fingerprint
        );
        assert_eq!(regenerated.theme.colors.accent, "#f6c177");
        assert_eq!(regenerated.theme.typography.body_size, 13.0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn watches_generated_and_source_files_without_reloading_unchanged_files() {
        let root = temp_root();
        let paths = ThemePaths::test(&root);
        write_sources(&root, "#9ccfd8");

        let generated = load_or_generate(&paths).unwrap();
        let mut cache = ThemeWatchCache::default();
        cache.refresh(theme_watch_paths(&paths, &generated.theme));
        assert!(!cache.changed());

        let contents = fs::read_to_string(&paths.output).unwrap();
        fs::write(&paths.output, format!("{contents}\n# live reload\n")).unwrap();
        assert!(cache.changed());

        cache.refresh(theme_watch_paths(&paths, &generated.theme));
        assert!(!cache.changed());

        write_sources(&root, "#f6c177ff");
        assert!(cache.changed());

        fs::remove_dir_all(root).unwrap();
    }
}
