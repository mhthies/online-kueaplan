use palette::{FromColor, IntoColor};

/// Set of display colors for a category, derived from the category's base color.
///
/// Provides background, border and text colors for dark and light theme. The colors try to mimic
/// the color grading of Bootstrap's semantic colors: https://getbootstrap.com/docs/5.3/customize/color/
pub struct CategoryColors {
    /// background color for the light theme (very bright, tinted according the base color)
    background_light: palette::Srgb<u8>,
    /// border color for the light theme
    border_light: palette::Srgb<u8>,
    /// text color for the light theme (black-ish)
    text_light: palette::Srgb<u8>,
    /// background color for the dark theme (very bright, tinted according the base color)
    background_dark: palette::Srgb<u8>,
    /// border color for the dark theme
    border_dark: palette::Srgb<u8>,
    /// background color for the dark theme (white-ish)
    text_dark: palette::Srgb<u8>,
}

impl CategoryColors {
    /// Generate a full set of colors (text, background, border; light and dark), corresponding to
    /// a user-selected base color of a category.
    pub fn from_base_color_hex(base_color_hex: &str) -> Result<Self, String> {
        let base_color: palette::Srgb<u8> = base_color_hex.parse().map_err(|e| format!("{}", e))?;
        let base_color_hsl: palette::Hsl = base_color.into_format::<f32>().into_color();

        Ok(Self {
            background_light: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.925,
            ))
            .into_format(),
            border_light: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.7,
            ))
            .into_format(),
            text_light: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.2,
            ))
            .into_format(),
            background_dark: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.10,
            ))
            .into_format(),
            border_dark: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.3,
            ))
            .into_format(),
            text_dark: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.8,
            ))
            .into_format(),
        })
    }

    /// Create a CSS style string, which sets all display colors to custom CSS properties, to be
    /// picked up by our CSS styling rules from the main.css file.
    pub fn as_css(&self) -> String {
        format!(
            "--category-bg:#{:x};--category-text:#{:x};--category-border:#{:x};\
            --category-bg-dark:#{:x};--category-text-dark:#{:x};--category-border-dark:#{:x};",
            self.background_light,
            self.text_light,
            self.border_light,
            self.background_dark,
            self.text_dark,
            self.border_dark,
        )
    }
}

/// Change luminance to target value +- 0.1 (based on the original luminance) and reduce
/// saturation after large changes of luminance.
///
/// New base luminance must not be < 0.075 or > 0.925.
fn change_color_luminance(color: &palette::Hsl, new_base_luminance: f32) -> palette::Hsl {
    debug_assert!(new_base_luminance >= 0.075);
    debug_assert!(new_base_luminance <= 0.925);
    let target_luminance = new_base_luminance + color.lightness * 0.15 - 0.075;
    let luminance_difference = (target_luminance - color.lightness).abs();
    let saturation_factor = 1.0 - luminance_difference * 0.6;
    let mut color = *color;
    color.lightness = target_luminance;
    color.saturation *= saturation_factor;
    color
}
