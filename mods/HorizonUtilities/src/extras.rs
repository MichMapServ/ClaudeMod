//! Native-to-the-.hmod features that use existing Horizon client imports.
//! This is an API *inside the guest*, not an unauthorized host import.
//! Never infer a pickup, saturation, durability or inventory snapshot here.

use crate::config::ExtrasConfig;
use horizon_mod_sdk::render::{FillShape, Point, Rect, Shape, TextShape};
use horizon_mod_sdk::{game, input, render};

pub const STORAGE_KEY: &str = "waypoints.v2";
pub const LEGACY_KEY: &str = "waypoints.v1";
const MAX_BYTES: usize = 4096;
const MAX_PROFILES: usize = 8;
const MAX_POINTS: usize = 8;
const COORD_LIMIT: f64 = 30_000_000.0;

fn safe_name(value: &str) -> bool {
    (1..=24).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn safe_dimension(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":_./-".contains(&b))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub name: String,
    pub dimension: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Waypoint {
    pub fn new(name: &str, dimension: &str, x: f64, y: f64, z: f64) -> Result<Self, String> {
        if !safe_name(name) {
            return Err("Nom de waypoint invalide (1-24 caractères : a-z, A-Z, 0-9, _, -)".into());
        }
        if !safe_dimension(dimension) {
            return Err("Dimension invalide".into());
        }
        if ![x, y, z]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= COORD_LIMIT)
        {
            return Err("Coordonnées invalides".into());
        }
        Ok(Self {
            name: name.to_string(),
            dimension: dimension.to_string(),
            x,
            y,
            z,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    pub points: Vec<Waypoint>,
    pub selected: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaypointStore {
    pub profiles: Vec<Profile>,
    pub active: Option<String>,
}

impl WaypointStore {
    pub fn create_profile(&mut self, name: &str) -> Result<(), String> {
        if !safe_name(name) {
            return Err("Nom de profil invalide".into());
        }
        if self.profiles.iter().any(|p| p.name == name) {
            return Err("Profil déjà présent".into());
        }
        if self.profiles.len() == MAX_PROFILES {
            return Err("Maximum 8 profils".into());
        }
        self.profiles.push(Profile {
            name: name.to_string(),
            points: Vec::new(),
            selected: None,
        });
        Ok(())
    }

    pub fn use_profile(&mut self, name: &str) -> Result<(), String> {
        if !self.profiles.iter().any(|p| p.name == name) {
            return Err("Profil inconnu. /utilities profile list".into());
        }
        self.active = Some(name.to_string());
        Ok(())
    }

    pub fn remove_profile(&mut self, name: &str) -> bool {
        let old = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        self.profiles.len() != old
    }

    pub fn current(&self) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| Some(p.name.as_str()) == self.active.as_deref())
    }

    pub fn current_mut(&mut self) -> Result<&mut Profile, String> {
        self.profiles
            .iter_mut()
            .find(|p| Some(p.name.as_str()) == self.active.as_deref())
            .ok_or_else(|| {
                "Choisis d'abord un profil : /utilities profile add NOM puis use NOM".into()
            })
    }

    pub fn add(&mut self, point: Waypoint) -> Result<(), String> {
        let profile = self.current_mut()?;
        if let Some(existing) = profile.points.iter_mut().find(|p| p.name == point.name) {
            *existing = point;
        } else if profile.points.len() >= MAX_POINTS {
            return Err("Maximum 8 waypoints par profil".into());
        } else {
            profile.points.push(point);
        }
        Ok(())
    }

    pub fn select(&mut self, name: &str) -> Result<(), String> {
        let profile = self.current_mut()?;
        if !profile.points.iter().any(|p| p.name == name) {
            return Err("Waypoint inconnu".into());
        }
        profile.selected = Some(name.to_string());
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<bool, String> {
        let profile = self.current_mut()?;
        let old = profile.points.len();
        profile.points.retain(|p| p.name != name);
        if profile.selected.as_deref() == Some(name) {
            profile.selected = None;
        }
        Ok(profile.points.len() != old)
    }

    pub fn selected(&self) -> Option<&Waypoint> {
        let p = self.current()?;
        p.points
            .iter()
            .find(|w| Some(w.name.as_str()) == p.selected.as_deref())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut lines = vec!["version=2".to_string()];
        // Deliberately DO NOT persist active profile across game sessions:
        // the SDK provides no stable world identity to bind the selection to.
        for profile in &self.profiles {
            lines.push(format!("profile={}", profile.name));
            if let Some(name) = &profile.selected {
                lines.push(format!("selected={}|{name}", profile.name));
            }
            for p in &profile.points {
                lines.push(format!(
                    "point={}|{}|{}|{:.3}|{:.3}|{:.3}",
                    profile.name, p.name, p.dimension, p.x, p.y, p.z
                ));
            }
        }
        (lines.join("\n") + "\n").into_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.len() > MAX_BYTES {
            return Self::default();
        }
        let Ok(src) = std::str::from_utf8(bytes) else {
            return Self::default();
        };
        if src.lines().next() != Some("version=2") {
            return Self::default();
        }
        let mut result = Self::default();
        let mut selections: Vec<(&str, &str)> = Vec::new();
        for line in src.lines().skip(1) {
            if let Some(name) = line.strip_prefix("profile=") {
                let _ = result.create_profile(name);
            } else if let Some(pair) = line.strip_prefix("selected=") {
                if let Some((profile, name)) = pair.split_once('|') {
                    selections.push((profile, name));
                }
            } else if let Some(data) = line.strip_prefix("point=") {
                let fields: Vec<_> = data.split('|').collect();
                if let [profile, name, dim, xs, ys, zs] = fields.as_slice() {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (xs.parse::<f64>(), ys.parse::<f64>(), zs.parse::<f64>())
                    {
                        if let Ok(w) = Waypoint::new(name, dim, x, y, z) {
                            if let Some(p) = result.profiles.iter_mut().find(|p| p.name == *profile)
                            {
                                if let Some(old) = p.points.iter_mut().find(|q| q.name == w.name) {
                                    *old = w;
                                } else if p.points.len() < MAX_POINTS {
                                    p.points.push(w);
                                }
                            }
                        }
                    }
                }
            }
        }
        for (profile, selected) in selections {
            if let Some(p) = result.profiles.iter_mut().find(|p| p.name == profile) {
                if p.points.iter().any(|w| w.name == selected) {
                    p.selected = Some(selected.to_string());
                }
            }
        }
        // Deliberately leave `active` as None until the player selects a
        // profile again in the current game session.
        result
    }

    /// Previous version was global across all worlds. Keep the data in
    /// an explicitly named `legacy` profile, but never activate it.
    pub fn migrate_v1(bytes: &[u8]) -> Self {
        let mut store = Self::default();
        if bytes.len() > MAX_BYTES {
            return store;
        }
        let Ok(src) = std::str::from_utf8(bytes) else {
            return store;
        };
        if src.lines().next() != Some("version=1") {
            return store;
        }
        let _ = store.create_profile("legacy");
        for line in src.lines().skip(1) {
            if let Some(data) = line.strip_prefix("point=") {
                let fields: Vec<_> = data.split('|').collect();
                if let [name, dim, xs, ys, zs] = fields.as_slice() {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (xs.parse::<f64>(), ys.parse::<f64>(), zs.parse::<f64>())
                    {
                        if let Ok(w) = Waypoint::new(name, dim, x, y, z) {
                            // Directly write the migration target without
                            // activating it automatically.
                            if let Some(p) = store.profiles.first_mut() {
                                if p.points.len() < MAX_POINTS {
                                    p.points.push(w);
                                }
                            }
                        }
                    }
                }
            }
        }
        store
    }
}

/// Minecraft yaw 0 = south, -90 = east, +90 = west.
pub fn relative_direction(yaw: f32, dx: f64, dz: f64) -> &'static str {
    if !yaw.is_finite() || !dx.is_finite() || !dz.is_finite() {
        return "direction inconnue";
    }
    if dx.abs() + dz.abs() < 0.5 {
        return "sur place";
    }
    let target_yaw = (-dx).atan2(dz).to_degrees();
    let diff = (target_yaw - f64::from(yaw) + 540.0).rem_euclid(360.0) - 180.0;
    if diff.abs() <= 35.0 {
        "devant"
    } else if diff.abs() >= 145.0 {
        "derrière"
    } else if diff < 0.0 {
        "droite"
    } else {
        "gauche"
    }
}

pub fn nav_hint(player: &game::Player, point: &Waypoint) -> String {
    if player.dimension != point.dimension {
        return format!("Cap {} : autre dimension", point.name);
    }
    let dx = point.x - player.x;
    let dy = point.y - player.y;
    let dz = point.z - player.z;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    format!(
        "Cap {} : {:.0} m, {}, Y {:+.0}",
        point.name,
        distance,
        relative_direction(player.yaw, dx, dz),
        dy
    )
}

#[derive(Default)]
pub struct Telemetry {
    previous: Option<input::HeldItem>,
    last_tick: Option<u64>,
    notice: Option<(String, u64)>,
    previous_lines: Vec<String>,
    previous_gui: (f32, f32),
    minimap: Option<bool>,
}

impl Telemetry {
    pub fn clear(&mut self) {
        if self.minimap.take().is_some() {
            render::set_minimap(None);
        }
        self.previous = None;
        self.last_tick = None;
        self.notice = None;
        self.previous_lines.clear();
        self.previous_gui = (0.0, 0.0);
        self.minimap = None;
    }

    pub fn sample(&mut self, held: Option<input::HeldItem>, tick: u64, track: bool) {
        if track {
            if let (Some(previous), Some(current), Some(last_tick)) =
                (&self.previous, &held, self.last_tick)
            {
                if tick == last_tick.saturating_add(1)
                    && previous.slot == current.slot
                    && previous.id == current.id
                    && previous.count != current.count
                {
                    let delta = i64::from(current.count) - i64::from(previous.count);
                    let item = current.id.rsplit(':').next().unwrap_or(&current.id);
                    self.notice = Some((
                        format!("Main {delta:+} {} (cause inconnue)", item.replace('_', " ")),
                        tick.saturating_add(60),
                    ));
                }
            }
        } else {
            self.notice = None;
        }
        self.previous = held;
        self.last_tick = Some(tick);
    }

    pub fn lines(
        &mut self,
        cfg: &ExtrasConfig,
        waypoints: &WaypointStore,
        player: &game::Player,
        tick: u64,
    ) -> Vec<String> {
        if !cfg.enabled {
            return Vec::new();
        }
        let mut rows = Vec::new();
        if cfg.performance {
            let frame = render::frame_stats();
            let (attack, use_clicks) = input::clicks_per_second();
            rows.push(format!(
                "FPS {} / 1% {} | CPS {attack}/{use_clicks}",
                frame.fps, frame.low
            ));
        }
        if cfg.survival_alerts {
            if player.food <= 6 {
                rows.push(format!("ATTENTION faim {}/20", player.food));
            }
            if player.health <= 6.0 {
                rows.push(format!("ATTENTION vie {:.0}/20", player.health));
            }
        }
        if cfg.navigation {
            if let Some(point) = waypoints.selected() {
                rows.push(nav_hint(player, point));
            }
        }
        if cfg.held_changes {
            if let Some((msg, end)) = &self.notice {
                if tick < *end {
                    rows.push(msg.clone());
                } else {
                    self.notice = None;
                }
            }
        }
        rows
    }

    pub fn panel(&mut self, lines: &[String], size: (f32, f32)) -> Vec<Shape> {
        if lines == self.previous_lines && self.previous_gui == size {
            return Vec::new();
        }
        self.previous_lines = lines.to_vec();
        self.previous_gui = size;
        if lines.is_empty() {
            return Vec::new();
        }
        let width = lines
            .iter()
            .map(|line| render::text_width(line))
            .fold(0.0_f32, f32::max)
            + 12.0;
        let width = width.max(24.0);
        let height = (lines.len() as f32 * 11.0) + 8.0;
        let left = ((size.0 - width) / 2.0).max(2.0);
        let top = 10.0;
        let mut out = vec![Shape::Fill(FillShape {
            rect: Rect {
                x: left,
                y: top,
                w: width,
                h: height,
            },
            color: 0x9900_0000,
        })];
        for (i, line) in lines.iter().enumerate() {
            out.push(Shape::Text(TextShape {
                text: line.clone(),
                at: Point {
                    x: left + 6.0,
                    y: top + 4.0 + i as f32 * 11.0,
                },
                color: if line.starts_with("ATTENTION") {
                    0xFF7777
                } else {
                    0xFFFFFF
                },
                centered: false,
                shadow: true,
            }));
        }
        out
    }

    pub fn minimap(&mut self, cfg: &ExtrasConfig) {
        let desired = if cfg.enabled && cfg.minimap {
            Some(cfg.rotate_map)
        } else {
            None
        };
        if self.minimap != desired {
            render::set_minimap(desired.map(|rotate| render::Minimap {
                corner: render::Corner::TopRight,
                size: 112.0,
                blocks_per_pixel: 2.0,
                rotate,
                round: true,
            }));
            self.minimap = desired;
        }
    }

    pub fn has_panel_changed(&self, lines: &[String], size: (f32, f32)) -> bool {
        self.previous_lines != lines || self.previous_gui != size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_is_explicit() {
        let mut db = WaypointStore::default();
        db.create_profile("worldA").unwrap();
        db.add(Waypoint::new("home", "minecraft:overworld", 1.0, 64.0, 2.0).unwrap())
            .unwrap_err();
        db.use_profile("worldA").unwrap();
        db.add(Waypoint::new("home", "minecraft:overworld", 1.0, 64.0, 2.0).unwrap())
            .unwrap();
        assert!(db.selected().is_none());
        db.select("home").unwrap();
        assert!(db.selected().is_some());
        db.create_profile("worldB").unwrap();
        db.use_profile("worldB").unwrap();
        assert!(db.selected().is_none());
    }

    #[test]
    fn round_trip() {
        let mut db = WaypointStore::default();
        db.create_profile("worldA").unwrap();
        db.use_profile("worldA").unwrap();
        db.add(Waypoint::new("home", "minecraft:overworld", 1.0, 64.0, 2.0).unwrap())
            .unwrap();
        db.select("home").unwrap();
        let decoded = WaypointStore::decode(&db.encode());
        assert_eq!(decoded.profiles, db.profiles);
        assert_eq!(decoded.active, None);
    }

    #[test]
    fn stale_profile_selection_is_never_loaded() {
        let content = b"version=2\nactive=wrongworld\nprofile=wrongworld\n";
        let loaded = WaypointStore::decode(content);
        assert_eq!(loaded.active, None);
    }

    #[test]
    fn v1_migration_is_not_active() {
        let old = b"version=1\nactive=base\npoint=base|minecraft:overworld|10|64|20\n";
        let db = WaypointStore::migrate_v1(old);
        assert_eq!(db.active, None);
        assert_eq!(db.profiles[0].name, "legacy");
        assert_eq!(db.profiles[0].points.len(), 1);
    }

    #[test]
    fn discontinuous_ticks_do_not_look_like_pickups() {
        let mut sample = Telemetry::default();
        let item = |count| input::HeldItem {
            id: "minecraft:stone".into(),
            count,
            slot: 0,
        };
        sample.sample(Some(item(10)), 15, true);
        sample.sample(Some(item(30)), 25, true);
        assert!(sample.notice.is_none());
        sample.sample(Some(item(31)), 26, true);
        assert!(sample.notice.as_ref().unwrap().0.contains("cause inconnue"));
        sample.sample(Some(item(32)), 27, false);
        assert!(sample.notice.is_none());
    }

    #[test]
    fn oversized_records_cannot_be_loaded() {
        let data = vec![b'x'; 4097];
        assert!(WaypointStore::decode(&data).profiles.is_empty());
        assert!(WaypointStore::migrate_v1(&data).profiles.is_empty());
    }

    #[test]
    fn unsafe_values_rejected() {
        assert!(Waypoint::new("bad|name", "minecraft:overworld", 0.0, 0.0, 0.0).is_err());
        assert!(Waypoint::new("base", "minecraft:overworld", f64::NAN, 0.0, 0.0).is_err());
        assert!(
            WaypointStore::decode(b"version=2\npoint=bad|x|y|z|0|0\n")
                .profiles
                .is_empty()
        );
    }

    #[test]
    fn directions_match_camera() {
        assert_eq!(relative_direction(0.0, 0.0, 5.0), "devant");
        assert_eq!(relative_direction(0.0, 5.0, 0.0), "droite");
        assert_eq!(relative_direction(0.0, -5.0, 0.0), "gauche");
        assert_eq!(relative_direction(-90.0, 5.0, 0.0), "devant");
    }

    #[test]
    fn profile_limits_enforced() {
        let mut db = WaypointStore::default();
        for i in 0..8 {
            db.create_profile(&format!("profile{i}")).unwrap();
        }
        assert!(db.create_profile("ninth").is_err());
    }
}
