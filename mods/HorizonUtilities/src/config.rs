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
/// Simultaneous pickup notifications, and how long each stays, in seconds.
pub const MAX_ENTRIES: [usize; 4] = [3, 5, 8, 10];
pub const SECONDS: [f32; 4] = [2.0, 3.0, 5.0, 8.0];

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

#[derive(Clone, Debug, PartialEq)]
pub struct ArmorConfig {
    pub enabled: bool,
    pub show_percent: bool,
    pub corner: Corner,
    /// Index into [`SCALES`].
    pub scale: usize,
}

impl Default for ArmorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_percent: true,
            corner: Corner::TopLeft,
            scale: 0,
        }
    }
}

impl ArmorConfig {
    pub fn scale_factor(&self) -> f32 {
        SCALES[self.scale.min(SCALES.len() - 1)]
    }

    pub fn cycle_scale(&mut self) {
        self.scale = (self.scale + 1) % SCALES.len();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PickupConfig {
    pub enabled: bool,
    pub corner: Corner,
    /// Index into [`MAX_ENTRIES`].
    pub max: usize,
    /// Index into [`SECONDS`].
    pub seconds: usize,
}

impl Default for PickupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            corner: Corner::TopRight,
            max: 1,
            seconds: 1,
        }
    }
}

impl PickupConfig {
    pub fn max_entries(&self) -> usize {
        MAX_ENTRIES[self.max.min(MAX_ENTRIES.len() - 1)]
    }

    pub fn lifetime(&self) -> f32 {
        SECONDS[self.seconds.min(SECONDS.len() - 1)]
    }

    pub fn cycle_max(&mut self) {
        self.max = (self.max + 1) % MAX_ENTRIES.len();
    }

    pub fn cycle_seconds(&mut self) {
        self.seconds = (self.seconds + 1) % SECONDS.len();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtrasConfig {
    pub enabled: bool,
    pub performance: bool,
    pub survival_alerts: bool,
    pub held_changes: bool,
    pub navigation: bool,
    pub minimap: bool,
    pub rotate_map: bool,
}

impl Default for ExtrasConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            performance: true,
            survival_alerts: true,
            held_changes: true,
            navigation: true,
            minimap: false,
            rotate_map: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub nutrition: NutritionConfig,
    pub extras: ExtrasConfig,
    pub armor: ArmorConfig,
    pub pickup: PickupConfig,
}

impl Config {
    pub fn encode(&self) -> Vec<u8> {
        let (n, a, p) = (&self.nutrition, &self.armor, &self.pickup);
        let lines = [
            format!("version={FORMAT_VERSION}"),
            format!("nutrition.enabled={}", n.enabled),
            format!("nutrition.bar={}", n.show_bar),
            format!("nutrition.preview={}", n.show_preview),
            format!("nutrition.held={}", n.show_held),
            format!("nutrition.corner={}", n.corner.id()),
            format!("nutrition.scale={}", n.scale),
            format!("nutrition.opacity={}", n.opacity),
            format!("armor.enabled={}", a.enabled),
            format!("armor.percent={}", a.show_percent),
            format!("armor.corner={}", a.corner.id()),
            format!("armor.scale={}", a.scale),
            format!("pickup.enabled={}", p.enabled),
            format!("pickup.corner={}", p.corner.id()),
            format!("pickup.max={}", p.max),
            format!("pickup.seconds={}", p.seconds),
            format!("extras.enabled={}", self.extras.enabled),
            format!("extras.performance={}", self.extras.performance),
            format!("extras.survival_alerts={}", self.extras.survival_alerts),
            format!("extras.held_changes={}", self.extras.held_changes),
            format!("extras.navigation={}", self.extras.navigation),
            format!("extras.minimap={}", self.extras.minimap),
            format!("extras.rotate_map={}", self.extras.rotate_map),
        ];
        let mut text = lines.join("\n");
        text.push('\n');
        text.into_bytes()
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
            match key {
                "version" => version_ok = value.parse::<u32>().ok() == Some(FORMAT_VERSION),
                "nutrition.enabled" => set_bool(&mut config.nutrition.enabled, value),
                "nutrition.bar" => set_bool(&mut config.nutrition.show_bar, value),
                "nutrition.preview" => set_bool(&mut config.nutrition.show_preview, value),
                "nutrition.held" => set_bool(&mut config.nutrition.show_held, value),
                "nutrition.corner" => set_corner(&mut config.nutrition.corner, value),
                "nutrition.scale" => set_index(&mut config.nutrition.scale, value, SCALES.len()),
                "nutrition.opacity" => {
                    set_index(&mut config.nutrition.opacity, value, OPACITIES.len())
                }
                "armor.enabled" => set_bool(&mut config.armor.enabled, value),
                "armor.percent" => set_bool(&mut config.armor.show_percent, value),
                "armor.corner" => set_corner(&mut config.armor.corner, value),
                "armor.scale" => set_index(&mut config.armor.scale, value, SCALES.len()),
                "pickup.enabled" => set_bool(&mut config.pickup.enabled, value),
                "pickup.corner" => set_corner(&mut config.pickup.corner, value),
                "pickup.max" => set_index(&mut config.pickup.max, value, MAX_ENTRIES.len()),
                "pickup.seconds" => set_index(&mut config.pickup.seconds, value, SECONDS.len()),
                "extras.enabled" => set_bool(&mut config.extras.enabled, value),
                "extras.performance" => set_bool(&mut config.extras.performance, value),
                "extras.survival_alerts" => set_bool(&mut config.extras.survival_alerts, value),
                "extras.held_changes" => set_bool(&mut config.extras.held_changes, value),
                "extras.navigation" => set_bool(&mut config.extras.navigation, value),
                "extras.minimap" => set_bool(&mut config.extras.minimap, value),
                "extras.rotate_map" => set_bool(&mut config.extras.rotate_map, value),
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

fn set_corner(slot: &mut Corner, value: &str) {
    if let Some(c) = Corner::from_id(value) {
        *slot = c;
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
        config.armor.show_percent = false;
        config.armor.corner = Corner::BottomLeft;
        config.pickup.max = 3;
        config.pickup.seconds = 0;
        config.pickup.enabled = false;
        config.extras.minimap = true;
        assert_eq!(Config::decode(&config.encode()), config);
    }

    #[test]
    fn old_settings_keep_new_defaults() {
        let data = b"version=1\nnutrition.enabled=false\n";
        let cfg = Config::decode(data);
        assert!(!cfg.nutrition.enabled);
        assert_eq!(cfg.extras, ExtrasConfig::default());
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
    fn a_settings_file_from_before_armor_and_pickups_still_loads() {
        let text = b"version=1\nnutrition.enabled=false\nnutrition.scale=2\n";
        let config = Config::decode(text);
        assert!(!config.nutrition.enabled);
        assert_eq!(config.nutrition.scale, 2);
        assert_eq!(config.armor, ArmorConfig::default());
        assert_eq!(config.pickup, PickupConfig::default());
    }

    #[test]
    fn bad_fields_fall_back_one_by_one() {
        let text = b"version=1\nnutrition.enabled=maybe\nnutrition.scale=99\nnutrition.corner=middle\nnutrition.bar=false\npickup.max=77\nunknown=1\nno equals sign\n";
        let config = Config::decode(text);
        let defaults = Config::default();
        assert_eq!(config.nutrition.enabled, defaults.nutrition.enabled);
        assert_eq!(config.nutrition.scale, defaults.nutrition.scale);
        assert_eq!(config.nutrition.corner, defaults.nutrition.corner);
        assert_eq!(config.pickup.max, defaults.pickup.max);
        assert!(!config.nutrition.show_bar);
    }

    #[test]
    fn cycles_stay_in_range() {
        let mut n = NutritionConfig::default();
        let mut p = PickupConfig::default();
        for _ in 0..10 {
            n.cycle_scale();
            n.cycle_opacity();
            p.cycle_max();
            p.cycle_seconds();
            assert!(n.scale < SCALES.len() && n.opacity < OPACITIES.len());
            assert!(p.max < MAX_ENTRIES.len() && p.seconds < SECONDS.len());
        }
        n.scale = 999;
        assert_eq!(n.scale_factor(), SCALES[SCALES.len() - 1]);
        p.max = 999;
        assert_eq!(p.max_entries(), MAX_ENTRIES[MAX_ENTRIES.len() - 1]);
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
