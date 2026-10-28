//! Persistent settings, independent of the SDK (testable without the game).
//!
//! Stored with `storage` under [`STORAGE_KEY`] as `key=value` lines. Decoding
//! never fails: anything unreadable falls back to the defaults, field by field.

/// `storage` key (`[a-z0-9_.-]{1,64}`).
pub const STORAGE_KEY: &str = "config.v1";
const FORMAT_VERSION: u32 = 1;
const MAX_BYTES: usize = 4096;

/// Bar and icon scales the settings page cycles through.
pub const SCALES: [f32; 4] = [1.0, 1.5, 2.0, 3.0];
/// Opacity percentages the settings page cycles through.
pub const OPACITIES: [u8; 4] = [25, 50, 75, 100];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub fn next(self) -> Self {
        match self {
            Corner::TopLeft => Corner::TopRight,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomRight => Corner::BottomLeft,
            Corner::BottomLeft => Corner::TopLeft,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Corner::TopLeft => "Haut gauche",
            Corner::TopRight => "Haut droite",
            Corner::BottomLeft => "Bas gauche",
            Corner::BottomRight => "Bas droite",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ]
        .into_iter()
        .find(|c| c.id() == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NutritionConfig {
    pub enabled: bool,
    /// The hunger bar and its "Faim n/20" line.
    pub show_bar: bool,
    /// The points the held food would restore, drawn on the bar.
    pub show_preview: bool,
    /// The held food's icon, name and points.
    pub show_held: bool,
    pub corner: Corner,
    /// Index into [`SCALES`].
    pub scale: usize,
    /// Index into [`OPACITIES`].
    pub opacity: usize,
}

impl Default for NutritionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_bar: true,
            show_preview: true,
            show_held: true,
            corner: Corner::BottomRight,
            scale: 0,
            opacity: OPACITIES.len() - 1,
        }
    }
}

impl NutritionConfig {
    pub fn scale_factor(&self) -> f32 {
        SCALES[self.scale.min(SCALES.len() - 1)]
    }

    pub fn opacity_percent(&self) -> u8 {
        OPACITIES[self.opacity.min(OPACITIES.len() - 1)]
    }

    pub fn cycle_scale(&mut self) {
        self.scale = (self.scale + 1) % SCALES.len();
    }

    pub fn cycle_opacity(&mut self) {
        self.opacity = (self.opacity + 1) % OPACITIES.len();
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub nutrition: NutritionConfig,
}

impl Config {
    pub fn encode(&self) -> Vec<u8> {
        let n = &self.nutrition;
        format!(
            "version={FORMAT_VERSION}\nnutrition.enabled={}\nnutrition.bar={}\nnutrition.preview={}\nnutrition.held={}\nnutrition.corner={}\nnutrition.scale={}\nnutrition.opacity={}\n",
            n.enabled,
            n.show_bar,
            n.show_preview,
            n.show_held,
            n.corner.id(),
            n.scale,
            n.opacity
        )
        .into_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Config {
        if bytes.len() > MAX_BYTES {
            return Config::default();
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Config::default();
        };
        let mut config = Config::default();
        let mut version_ok = false;
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            let n = &mut config.nutrition;
            match key {
                "version" => version_ok = value.parse::<u32>().ok() == Some(FORMAT_VERSION),
                "nutrition.enabled" => set_bool(&mut n.enabled, value),
                "nutrition.bar" => set_bool(&mut n.show_bar, value),
                "nutrition.preview" => set_bool(&mut n.show_preview, value),
                "nutrition.held" => set_bool(&mut n.show_held, value),
                "nutrition.corner" => {
                    if let Some(c) = Corner::from_id(value) {
                        n.corner = c;
                    }
                }
                "nutrition.scale" => set_index(&mut n.scale, value, SCALES.len()),
                "nutrition.opacity" => set_index(&mut n.opacity, value, OPACITIES.len()),
                _ => {}
            }
        }
        if version_ok {
            config
        } else {
            Config::default()
        }
    }
}

fn set_bool(slot: &mut bool, value: &str) {
    match value {
        "true" => *slot = true,
        "false" => *slot = false,
        _ => {}
    }
}

fn set_index(slot: &mut usize, value: &str, len: usize) {
    if let Ok(i) = value.parse::<usize>() {
        if i < len {
            *slot = i;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut config = Config::default();
        config.nutrition.enabled = false;
        config.nutrition.corner = Corner::TopLeft;
        config.nutrition.scale = 2;
        config.nutrition.opacity = 1;
        assert_eq!(Config::decode(&config.encode()), config);
    }

    #[test]
    fn garbage_gives_defaults() {
        assert_eq!(Config::decode(b"\xff\xfe"), Config::default());
        assert_eq!(Config::decode(b""), Config::default());
        assert_eq!(
            Config::decode(&vec![b'a'; MAX_BYTES + 1]),
            Config::default()
        );
    }

    #[test]
    fn unknown_version_gives_defaults() {
        let text = b"version=2\nnutrition.enabled=false\n";
        assert_eq!(Config::decode(text), Config::default());
        let no_version = b"nutrition.enabled=false\n";
        assert_eq!(Config::decode(no_version), Config::default());
    }

    #[test]
    fn bad_fields_fall_back_one_by_one() {
        let text = b"version=1\nnutrition.enabled=maybe\nnutrition.scale=99\nnutrition.corner=middle\nnutrition.bar=false\nunknown=1\nno equals sign\n";
        let config = Config::decode(text);
        let defaults = Config::default();
        assert_eq!(config.nutrition.enabled, defaults.nutrition.enabled);
        assert_eq!(config.nutrition.scale, defaults.nutrition.scale);
        assert_eq!(config.nutrition.corner, defaults.nutrition.corner);
        assert!(!config.nutrition.show_bar);
    }

    #[test]
    fn cycles_stay_in_range() {
        let mut n = NutritionConfig::default();
        for _ in 0..10 {
            n.cycle_scale();
            n.cycle_opacity();
            assert!(n.scale < SCALES.len() && n.opacity < OPACITIES.len());
        }
        n.scale = 999;
        assert_eq!(n.scale_factor(), SCALES[SCALES.len() - 1]);
    }

    #[test]
    fn corners_cycle_through_all_four() {
        let mut c = Corner::TopLeft;
        let mut seen = vec![c];
        for _ in 0..3 {
            c = c.next();
            seen.push(c);
        }
        seen.sort_by_key(|c| c.id());
        seen.dedup();
        assert_eq!(seen.len(), 4);
        assert_eq!(c.next(), Corner::TopLeft);
    }
}
