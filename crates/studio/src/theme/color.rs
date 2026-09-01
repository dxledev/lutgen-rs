use egui::Color32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemeColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl ThemeColor {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let hex = value
            .trim()
            .strip_prefix('#')
            .ok_or_else(|| format!("theme color must start with '#': {value}"))?;
        if hex.len() != 6 && hex.len() != 8 {
            return Err(format!(
                "theme color must use #RRGGBB or #RRGGBBAA: {value}"
            ));
        }

        let channel = |range: std::ops::Range<usize>| {
            u8::from_str_radix(&hex[range], 16).map_err(|_| format!("invalid theme color: {value}"))
        };

        Ok(Self {
            red: channel(0..2)?,
            green: channel(2..4)?,
            blue: channel(4..6)?,
            alpha: if hex.len() == 8 { channel(6..8)? } else { 255 },
        })
    }

    pub fn to_hex(self) -> String {
        if self.alpha == 255 {
            format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
        } else {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                self.red, self.green, self.blue, self.alpha
            )
        }
    }

    pub fn to_egui(self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.red, self.green, self.blue, self.alpha)
    }

    pub fn contrast_ratio(self, background: Self) -> f32 {
        let foreground_luminance = self.relative_luminance();
        let background_luminance = background.relative_luminance();
        let lighter = foreground_luminance.max(background_luminance);
        let darker = foreground_luminance.min(background_luminance);
        (lighter + 0.05) / (darker + 0.05)
    }

    pub fn ensure_contrast(self, background: Self, minimum_ratio: f32) -> Self {
        if self.contrast_ratio(background) >= minimum_ratio {
            return self;
        }

        let target =
            if Self::WHITE.contrast_ratio(background) >= Self::BLACK.contrast_ratio(background) {
                Self::WHITE
            } else {
                Self::BLACK
            };

        (1..=100)
            .map(|step| self.mix(target, step as f32 / 100.0))
            .find(|candidate| candidate.contrast_ratio(background) >= minimum_ratio)
            .unwrap_or(target)
    }

    pub fn ensure_contrast_against(self, backgrounds: &[Self], minimum_ratio: f32) -> Self {
        if backgrounds
            .iter()
            .all(|background| self.contrast_ratio(*background) >= minimum_ratio)
        {
            return self;
        }

        let minimum_contrast = |candidate: Self| {
            backgrounds
                .iter()
                .map(|background| candidate.contrast_ratio(*background))
                .fold(f32::INFINITY, f32::min)
        };
        let target = if minimum_contrast(Self::WHITE) >= minimum_contrast(Self::BLACK) {
            Self::WHITE
        } else {
            Self::BLACK
        };

        (1..=100)
            .map(|step| self.mix(target, step as f32 / 100.0))
            .find(|candidate| {
                backgrounds
                    .iter()
                    .all(|background| candidate.contrast_ratio(*background) >= minimum_ratio)
            })
            .unwrap_or(target)
    }

    pub fn best_text_color(background: Self) -> Self {
        if Self::WHITE.contrast_ratio(background) >= Self::BLACK.contrast_ratio(background) {
            Self::WHITE
        } else {
            Self::BLACK
        }
    }

    pub fn with_alpha(self, alpha: u8) -> Self {
        Self { alpha, ..self }
    }

    pub fn is_dark(self) -> bool {
        self.relative_luminance() < 0.35
    }

    fn mix(self, target: Self, amount: f32) -> Self {
        let mix_channel = |start: u8, end: u8| {
            (start as f32 + (end as f32 - start as f32) * amount).round() as u8
        };
        Self {
            red: mix_channel(self.red, target.red),
            green: mix_channel(self.green, target.green),
            blue: mix_channel(self.blue, target.blue),
            alpha: self.alpha,
        }
    }

    fn relative_luminance(self) -> f32 {
        let linear = |channel: u8| {
            let value = channel as f32 / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.red) + 0.7152 * linear(self.green) + 0.0722 * linear(self.blue)
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeColor;

    #[test]
    fn parses_rgb_and_rgba_hex() {
        assert_eq!(ThemeColor::parse("#191724").unwrap().to_hex(), "#191724");
        assert_eq!(
            ThemeColor::parse("#19172480").unwrap().to_hex(),
            "#19172480"
        );
    }

    #[test]
    fn raises_low_contrast_text_to_requested_ratio() {
        let background = ThemeColor::parse("#191724").unwrap();
        let muted = ThemeColor::parse("#403e49").unwrap();
        let corrected = muted.ensure_contrast(background, 4.5);

        assert!(corrected.contrast_ratio(background) >= 4.5);
    }
}
