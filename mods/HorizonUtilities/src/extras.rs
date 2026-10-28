//! Features supported by Horizon's public mod API: waypoint navigation,
//! performance display, survival warnings and *held-stack change* hints.
//! This module intentionally does not pretend to read equipment/inventory,
//! saturation or pickup events (none is exposed to a .hmod).
use crate::config::ExtrasConfig;
use horizon_mod_sdk::{game, input, render};
use horizon_mod_sdk::render::{FillShape, Point, Rect, Shape, TextShape};

pub const WAYPOINTS_KEY: &str = "waypoints.v1";
const MAX_WAYPOINTS: usize = 8;
const LIMIT: f64 = 30_000_000.0;
const MAX_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub name: String,
    pub dimension: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Waypoint {
    pub fn at(name: &str, dimension: &str, x: f64, y: f64, z: f64) -> Result<Self, String> {
        if !valid_name(name) {
            return Err("Nom : 1-24 caractères ASCII (lettres, chiffres, _ ou -)".into());
        }
        if dimension.is_empty()
            || dimension.len() > 128
            || !dimension
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b":_./-".contains(&c))
        {
            return Err("Dimension invalide".into());
        }
        if ![x, y, z].iter().all(|v| v.is_finite() && v.abs() <= LIMIT) {
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

    pub fn from_player(name: &str, player: &game::Player) -> Result<Self, String> {
        Self::at(name, &player.dimension, player.x, player.y, player.z)
    }
}

fn valid_name(name: &str) -> bool {
    (1..=24).contains(&name.len())
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Waypoints {
    pub points: Vec<Waypoint>,
    pub active: Option<String>,
}

impl Waypoints {
    pub fn add(&mut self, point: Waypoint) -> Result<(), String> {
        if let Some(existing) = self.points.iter_mut().find(|p| p.name == point.name) {
            *existing = point;
        } else if self.points.len() == MAX_WAYPOINTS {
            return Err("Maximum 8 points de repère ; supprimez-en un".into());
        } else {
            self.points.push(point);
        }
        Ok(())
    }

    pub fn select(&mut self, name: &str) -> Result<(), String> {
        if self.points.iter().any(|p| p.name == name) {
            self.active = Some(name.to_string());
            Ok(())
        } else {
            Err(format!("Point inconnu : {name}"))
        }
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.points.len();
        self.points.retain(|p| p.name != name);
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        before != self.points.len()
    }

    pub fn selected(&self) -> Option<&Waypoint> {
        self.points.iter().find(|p| Some(p.name.as_str()) == self.active.as_deref())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut result = String::from("version=1\n");
        if let Some(name) = &self.active {
            result.push_str(&format!("active={name}\n"));
        }
        for p in &self.points {
            result.push_str(&format!(
                "point={}|{}|{:.3}|{:.3}|{:.3}\n",
                p.name, p.dimension, p.x, p.y, p.z
            ));
        }
        result.into_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.len() > MAX_BYTES {
            return Self::default();
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Self::default();
        };
        if !text.lines().any(|line| line == "version=1") {
            return Self::default();
        }
        let mut out = Self::default();
        let mut active = None;
        for line in text.lines() {
            if let Some(name) = line.strip_prefix("active=") {
                if valid_name(name) {
                    active = Some(name.to_string());
                }
            }
            if let Some(data) = line.strip_prefix("point=") {
                let fields: Vec<&str> = data.split('|').collect();
                if let [name, dimension, x, y, z] = fields.as_slice() {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (x.parse::<f64>(), y.parse::<f64>(), z.parse::<f64>())
                    {
                        if let Ok(p) = Waypoint::at(name, dimension, x, y, z) {
                            let _ = out.add(p);
                        }
                    }
                }
            }
        }
        if let Some(name) = active {
            let _ = out.select(&name);
        }
        out
    }

    pub fn list(&self) -> String {
        if self.points.is_empty() {
            return "Aucun waypoint. /utilities wp add NOM".into();
        }
        self.points
            .iter()
            .map(|p| {
                format!(
                    "{}{} ({:.0}, {:.0}, {:.0}, {})",
                    if self.active.as_deref() == Some(p.name.as_str()) {
                        "*"
                    } else {
                        ""
                    },
                    p.name,
                    p.x,
                    p.y,
                    p.z,
                    p.dimension
                )
            })
            .collect::<Vec<_>>()
            .join(" ; ")
    }
}

/// Minecraft yaw 0 faces south, -90 east, +90 west.
pub fn relative_direction(yaw: f32, dx: f64, dz: f64) -> &'static str {
    if dx.abs() + dz.abs() < 0.5 {
        return "sur place";
    }
    let target_yaw = (-dx).atan2(dz).to_degrees();
    let diff = (target_yaw - f64::from(yaw) + 540.0).rem_euclid(360.0) - 180.0;
    if diff.abs() <= 35.0 {
        "devant"
    } else if diff.abs() >= 145.0 {
        "derriere"
    } else if diff < 0.0 {
        "droite"
    } else {
        "gauche"
    }
}

pub fn waypoint_hint(player: &game::Player, point: &Waypoint) -> String {
    if player.dimension != point.dimension {
        return format!("Cap {} : autre dimension", point.name);
    }
    let (dx, dy, dz) = (point.x - player.x, point.y - player.y, point.z - player.z);
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
pub struct Runtime {
    last_held: Option<input::HeldItem>,
    notice: Option<(String, u64)>,
    lines: Vec<String>,
    map: Option<(bool, bool)>,
}

impl Runtime {
    pub fn hand(&mut self, held: Option<input::HeldItem>, tick: u64, track: bool) {
        if track {
            if let (Some(before), Some(after)) = (&self.last_held, &held) {
                if before.slot == after.slot && before.id == after.id && before.count != after.count {
                    let delta = i64::from(after.count) - i64::from(before.count);
                    let label = after.id.rsplit(':').next().unwrap_or(&after.id).replace('_', " ");
                    self.notice = Some((
                        format!("Main {delta:+} {label} (cause inconnue)"),
                        tick.saturating_add(60),
                    ));
                }
            }
        } else {
            self.notice = None;
        }
        self.last_held = held;
    }

    pub fn clear(&mut self) {
        if !self.lines.is_empty() {
            render::set_hud(&[]);
            self.lines.clear();
        }
        if self.map.take().is_some() {
            render::set_minimap(None);
        }
        self.last_held = None;
        self.notice = None;
    }

    pub fn refresh(
        &mut self,
        cfg: &ExtrasConfig,
        waypoints: &Waypoints,
        player: &game::Player,
        tick: u64,
    ) {
        let wanted_map = if cfg.enabled && cfg.minimap {
            Some((cfg.minimap, cfg.rotate_map))
        } else {
            None
        };
        if self.map != wanted_map {
            render::set_minimap(wanted_map.map(|(_, rotate)| render::Minimap {
                corner: render::Corner::TopRight,
                size: 112.0,
                blocks_per_pixel: 2.0,
                rotate,
                round: true,
            }));
            self.map = wanted_map;
        }

        let mut lines = Vec::new();
        if cfg.enabled {
            if cfg.performance {
                let rate = render::frame_stats();
                let (attack, use_clicks) = input::clicks_per_second();
                lines.push(format!(
                    "FPS {} | 1% {} | CPS {}/{}",
                    rate.fps, rate.low, attack, use_clicks
                ));
            }
            if cfg.survival_alerts && player.food <= 6 {
                lines.push(format!("ATTENTION : faim {}/20", player.food));
            }
            if cfg.survival_alerts && player.health <= 6.0 {
                lines.push(format!("ATTENTION : vie {:.0}/20", player.health));
            }
            if cfg.navigation {
                if let Some(point) = waypoints.selected() {
                    lines.push(waypoint_hint(player, point));
                }
            }
            if cfg.held_changes {
                if let Some((message, expires)) = &self.notice {
                    if tick < *expires {
                        lines.push(message.clone());
                    } else {
                        self.notice = None;
                    }
                }
            }
        }
        if lines == self.lines {
            return;
        }
        if lines.is_empty() {
            render::set_hud(&[]);
        } else {
            let (screen_w, _) = render::gui_size();
            let width = lines
                .iter()
                .map(|s| render::text_width(s))
                .fold(0.0_f32, f32::max)
                + 12.0;
            let height = lines.len() as f32 * 11.0 + 8.0;
            let left = ((screen_w - width) / 2.0).max(2.0);
            let top = 8.0;
            let mut shapes = vec![Shape::Fill(FillShape {
                rect: Rect {
                    x: left,
                    y: top,
                    w: width,
                    h: height,
                },
                color: 0x99000000,
            })];
            for (i, line) in lines.iter().enumerate() {
                shapes.push(Shape::Text(TextShape {
                    text: line.clone(),
                    at: Point {
                        x: left + 6.0,
                        y: top + 4.0 + i as f32 * 11.0,
                    },
                    color: if line.starts_with("ATTENTION") {
                        0xFF8888
                    } else {
                        0xFFFFFF
                    },
                    centered: false,
                    shadow: true,
                }));
            }
            render::set_hud(&shapes);
        }
        self.lines = lines;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waypoints_roundtrip_and_select() {
        let mut points = Waypoints::default();
        points
            .add(Waypoint::at("base", "minecraft:overworld", 12.0, 64.0, -4.0).unwrap())
            .unwrap();
        points.select("base").unwrap();
        assert_eq!(Waypoints::decode(&points.encode()), points);
        assert!(points.remove("base"));
        assert!(points.active.is_none());
    }

    #[test]
    fn invalid_waypoints_are_rejected() {
        assert!(Waypoint::at("bad name", "minecraft:overworld", 0.0, 1.0, 2.0).is_err());
        assert!(Waypoint::at("abc", "minecraft:overworld", f64::NAN, 1.0, 2.0).is_err());
        assert!(Waypoint::at("abc", "minecraft:overworld", 30_000_001.0, 1.0, 2.0).is_err());
        assert_eq!(Waypoints::decode(b"other=2"), Waypoints::default());
    }

    #[test]
    fn direction_is_correct_for_minecraft_yaw() {
        assert_eq!(relative_direction(0.0, 0.0, 10.0), "devant");
        assert_eq!(relative_direction(0.0, 10.0, 0.0), "droite");
        assert_eq!(relative_direction(0.0, -10.0, 0.0), "gauche");
        assert_eq!(relative_direction(0.0, 0.0, -10.0), "derriere");
        assert_eq!(relative_direction(-90.0, 10.0, 0.0), "devant");
    }

    #[test]
    fn capacity_and_replacement() {
        let mut w = Waypoints::default();
        for i in 0..8 {
            w.add(Waypoint::at(&format!("w{i}"), "minecraft:overworld", i as f64, 1.0, 2.0).unwrap()).unwrap();
        }
        assert!(w.add(Waypoint::at("too-many", "minecraft:overworld", 0.0, 0.0, 0.0).unwrap()).is_err());
        w.add(Waypoint::at("w0", "minecraft:overworld", 9.0, 1.0, 2.0).unwrap()).unwrap();
        assert_eq!(w.points.len(), 8);
    }
}
