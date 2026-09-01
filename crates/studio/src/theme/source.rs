use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::color::ThemeColor;

#[derive(Clone, Debug)]
pub struct ThemePaths {
    pub output: PathBuf,
    chromack_config: PathBuf,
    dashboard_theme: PathBuf,
    dashboard_style: PathBuf,
    wardnc_config: PathBuf,
    current_theme_dir: PathBuf,
}

impl ThemePaths {
    pub fn from_env() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        Self {
            output: env_path("LUTGEN_STUDIO_THEME_FILE")
                .unwrap_or_else(|| workspace_root().join("theme.toml")),
            chromack_config: env_path("LUTGEN_STUDIO_CHROMACK_CONFIG")
                .unwrap_or_else(|| config_dir.join("chromack/config.toml")),
            dashboard_theme: env_path("LUTGEN_STUDIO_DASHBOARD_THEME")
                .unwrap_or_else(|| config_dir.join("dashboard/theme.qml")),
            dashboard_style: env_path("LUTGEN_STUDIO_DASHBOARD_STYLE")
                .unwrap_or_else(|| config_dir.join("dashboard/style.qml")),
            wardnc_config: env_path("LUTGEN_STUDIO_WARDNC_CONFIG")
                .unwrap_or_else(|| config_dir.join("wardnc/config.toml")),
            current_theme_dir: env_path("LUTGEN_STUDIO_CURRENT_THEME_DIR")
                .unwrap_or_else(|| config_dir.join("themes/current")),
        }
    }

    pub fn watch_paths(&self) -> Vec<PathBuf> {
        vec![
            self.output.clone(),
            self.chromack_config.clone(),
            self.dashboard_theme.clone(),
            self.dashboard_style.clone(),
            self.wardnc_config.clone(),
            self.current_theme_dir.clone(),
        ]
    }

    #[cfg(test)]
    pub fn test(root: &Path) -> Self {
        Self {
            output: root.join("theme.toml"),
            chromack_config: root.join("chromack/config.toml"),
            dashboard_theme: root.join("dashboard/theme.qml"),
            dashboard_style: root.join("dashboard/style.qml"),
            wardnc_config: root.join("wardnc/config.toml"),
            current_theme_dir: root.join("themes/current"),
        }
    }
}

#[derive(Debug)]
pub struct SourceSnapshot {
    pub active_theme: String,
    pub chromack: HashMap<String, String>,
    pub dashboard_theme: HashMap<String, String>,
    pub dashboard_style: HashMap<String, String>,
    pub wardnc: HashMap<String, String>,
    pub fingerprint: String,
    pub source_paths: Vec<String>,
}

impl SourceSnapshot {
    pub fn collect(paths: &ThemePaths) -> Result<Self, String> {
        let mut collector = SourceCollector::default();

        let chromack_config = collector.read_toml(&paths.chromack_config)?;
        let chromack_base = paths
            .chromack_config
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let chromack_style = config_path(&chromack_config, "style_css", chromack_base)?;
        let chromack_colors = config_path(&chromack_config, "colors_css", chromack_base)?;
        let chromack_material = config_path(&chromack_config, "material_css", chromack_base).ok();
        let mut chromack = HashMap::new();
        collector.load_css(&chromack_style, &mut chromack)?;
        collector.load_css(&chromack_colors, &mut chromack)?;
        if let Some(path) = chromack_material {
            collector.load_css(&path, &mut chromack)?;
        }

        let dashboard_theme = parse_qml_properties(&collector.read(&paths.dashboard_theme)?);
        let dashboard_style = parse_qml_properties(&collector.read(&paths.dashboard_style)?);

        let wardnc_config = collector.read_toml(&paths.wardnc_config)?;
        let wardnc_base = paths
            .wardnc_config
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let wardnc_style = config_path(&wardnc_config, "style_css", wardnc_base)?;
        let wardnc_colors = config_path(&wardnc_config, "colors_css", wardnc_base)?;
        let mut wardnc = HashMap::new();
        collector.load_css(&wardnc_colors, &mut wardnc)?;
        let global_theme_variables = paths.current_theme_dir.join("ward.css");
        if global_theme_variables.is_file() {
            collector.load_css(&global_theme_variables, &mut wardnc)?;
        }
        collector.load_css(&wardnc_style, &mut wardnc)?;

        let active_theme = fs::canonicalize(&paths.current_theme_dir)
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "system".to_string());

        Ok(Self {
            active_theme,
            chromack,
            dashboard_theme,
            dashboard_style,
            wardnc,
            fingerprint: collector.fingerprint(),
            source_paths: collector
                .files
                .iter()
                .map(|file| file.path.display().to_string())
                .collect(),
        })
    }

    pub fn color(&self, candidates: &[ThemeValue<'_>]) -> Option<ThemeColor> {
        candidates.iter().find_map(|candidate| {
            let value = match candidate.source {
                ThemeSource::Chromack => resolved_css_value(&self.chromack, candidate.name),
                ThemeSource::DashboardTheme => self.dashboard_theme.get(candidate.name).cloned(),
                ThemeSource::DashboardStyle => self.dashboard_style.get(candidate.name).cloned(),
                ThemeSource::WardNc => resolved_css_value(&self.wardnc, candidate.name),
            }?;
            ThemeColor::parse(&value).ok()
        })
    }

    pub fn number(&self, candidates: &[ThemeValue<'_>]) -> Option<f32> {
        candidates.iter().find_map(|candidate| {
            let value = match candidate.source {
                ThemeSource::Chromack => resolved_css_value(&self.chromack, candidate.name),
                ThemeSource::DashboardTheme => self.dashboard_theme.get(candidate.name).cloned(),
                ThemeSource::DashboardStyle => self.dashboard_style.get(candidate.name).cloned(),
                ThemeSource::WardNc => resolved_css_value(&self.wardnc, candidate.name),
            }?;
            parse_number(&value)
        })
    }
}

#[derive(Clone, Copy)]
pub enum ThemeSource {
    Chromack,
    DashboardTheme,
    DashboardStyle,
    WardNc,
}

#[derive(Clone, Copy)]
pub struct ThemeValue<'a> {
    pub source: ThemeSource,
    pub name: &'a str,
}

impl<'a> ThemeValue<'a> {
    pub const fn new(source: ThemeSource, name: &'a str) -> Self {
        Self { source, name }
    }
}

#[derive(Default)]
struct SourceCollector {
    files: Vec<SourceFile>,
    visited_css: HashSet<PathBuf>,
}

struct SourceFile {
    path: PathBuf,
    contents: String,
}

impl SourceCollector {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        let contents = fs::read_to_string(path)
            .map_err(|error| format!("failed to read theme source {}: {error}", path.display()))?;
        let tracked_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !self.files.iter().any(|file| file.path == tracked_path) {
            self.files.push(SourceFile {
                path: tracked_path,
                contents: contents.clone(),
            });
        }
        Ok(contents)
    }

    fn read_toml(&mut self, path: &Path) -> Result<toml::Value, String> {
        let contents = self.read(path)?;
        toml::from_str(&contents)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }

    fn load_css(
        &mut self,
        path: &Path,
        variables: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        let identity = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !self.visited_css.insert(identity) {
            return Ok(());
        }

        let contents = self.read(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        for import in css_imports(&contents) {
            let import_path = expand_path(&import);
            let import_path = if import_path.is_absolute() {
                import_path
            } else {
                base_dir.join(import_path)
            };
            self.load_css(&import_path, variables)?;
        }
        variables.extend(parse_css_variables(&contents));
        Ok(())
    }

    fn fingerprint(&self) -> String {
        let mut hash = 0xcbf29ce484222325_u64;
        for file in &self.files {
            for byte in file.path.to_string_lossy().bytes().chain([0]) {
                hash = fnv_step(hash, byte);
            }
            for byte in file.contents.bytes().chain([0]) {
                hash = fnv_step(hash, byte);
            }
        }
        format!("{hash:016x}")
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name).ok().map(|path| expand_path(&path))
}

fn config_path(config: &toml::Value, key: &str, base_dir: &Path) -> Result<PathBuf, String> {
    let path = config
        .get("paths")
        .and_then(|paths| paths.get(key))
        .and_then(toml::Value::as_str)
        .map(expand_path)
        .ok_or_else(|| format!("missing paths.{key} in desktop application config"))?;
    Ok(if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    })
}

fn expand_path(value: &str) -> PathBuf {
    let mut expanded = value.to_string();
    if let Some(home) = dirs::home_dir() {
        if expanded == "~" {
            expanded = home.display().to_string();
        } else if let Some(remainder) = expanded.strip_prefix("~/") {
            expanded = home.join(remainder).display().to_string();
        }
    }

    let mut variables = std::env::vars().collect::<Vec<_>>();
    variables.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    for (name, value) in variables {
        expanded = expanded.replace(&format!("${{{name}}}"), &value);
        expanded = expanded.replace(&format!("${name}"), &value);
    }
    PathBuf::from(expanded)
}

fn parse_css_variables(contents: &str) -> HashMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let declaration = line.strip_prefix("--")?;
            let (name, value) = declaration.split_once(':')?;
            Some((
                name.trim().to_string(),
                value.trim().trim_end_matches(';').trim().to_string(),
            ))
        })
        .collect()
}

fn css_imports(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with("@import") {
                return None;
            }
            let quote_index = line.find(['\'', '"'])?;
            let quote = line.as_bytes()[quote_index] as char;
            let remainder = &line[quote_index + 1..];
            let end = remainder.find(quote)?;
            Some(remainder[..end].to_string())
        })
        .collect()
}

fn parse_qml_properties(contents: &str) -> HashMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let declaration = line.strip_prefix("property ")?;
            let (key, value) = declaration.split_once(':')?;
            let name = key.split_whitespace().last()?;
            Some((name.to_string(), value.trim().trim_matches('"').to_string()))
        })
        .collect()
}

fn resolved_css_value(variables: &HashMap<String, String>, name: &str) -> Option<String> {
    let mut value = variables.get(name)?.clone();
    let mut visited = HashSet::new();
    while let Some(reference) = value
        .strip_prefix("var(--")
        .and_then(|value| value.strip_suffix(')'))
    {
        let reference = reference.split(',').next()?.trim();
        if !visited.insert(reference.to_string()) {
            return None;
        }
        value = variables.get(reference)?.clone();
    }
    Some(value)
}

fn parse_number(value: &str) -> Option<f32> {
    value
        .trim()
        .trim_end_matches("ms")
        .trim_end_matches("px")
        .trim()
        .parse()
        .ok()
}

fn fnv_step(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
}

#[cfg(test)]
mod tests {
    use super::{parse_css_variables, parse_qml_properties, resolved_css_value};

    #[test]
    fn resolves_css_variable_aliases() {
        let variables =
            parse_css_variables(":root {\n--surface: #191724;\n--panel: var(--surface);\n}");

        assert_eq!(
            resolved_css_value(&variables, "panel").as_deref(),
            Some("#191724")
        );
    }

    #[test]
    fn reads_qml_theme_properties() {
        let values =
            parse_qml_properties("property color bg: \"#191724\"\nproperty real opacity: 1.0");

        assert_eq!(values.get("bg").map(String::as_str), Some("#191724"));
        assert_eq!(values.get("opacity").map(String::as_str), Some("1.0"));
    }
}
