//! Horizon Utilities for Horizon.
//!
//! Standard build (official SDK): the nutrition HUD with the real hunger
//! level. Extended build (feature `native`, a Horizon extended as
//! `ENGINE_PATCH.md` says): also real saturation, the held food's real
//! values, armor durability, pickup notifications and inventory sorting.
//! What a mode cannot serve is listed in the settings page, never faked.
mod armor;
mod config;
mod extras;
mod food;
mod hud;
#[cfg(feature = "native")]
mod native;
mod nutrition;
mod pickup;
mod provider;
mod settings;
mod unavailable;

use config::{Config, NutritionConfig, STORAGE_KEY};
use extras::{Telemetry, Waypoint, WaypointStore};
use horizon_mod_sdk::render::Shape;
use horizon_mod_sdk::{Mod, Registration, game, info, input, render, storage, ui, warn};
use nutrition::Snapshot;
use provider::{Provider, SortRegion, Stack};

/// Name of the key `input::register_key` hands back in `key-down`.
const KEY_TOGGLE: &str = "toggle_nutrition";

#[cfg(feature = "native")]
fn provider() -> Box<dyn Provider> {
    Box::new(native::NativeProvider)
}

#[cfg(not(feature = "native"))]
fn provider() -> Box<dyn Provider> {
    Box::new(provider::StandardProvider)
}

struct Utilities {
    config: Config,
    provider: Box<dyn Provider>,
    /// What the nutrition panel last showed, so an unchanged one costs no call.
    nutrition_drawn: Option<(Snapshot, NutritionConfig)>,
    equipment: Option<Vec<Option<Stack>>>,
    armor_shapes: Vec<Shape>,
    pickup_shapes: Vec<Shape>,
    extras_shapes: Vec<Shape>,
    telemetry: Telemetry,
    waypoints: WaypointStore,
    /// Shapes were sent with `set_hud` and not cleared since.
    hud_sent: bool,
    notifier: pickup::Notifier,
    gui: (f32, f32),
}

fn width_of(text: &str) -> f32 {
    render::text_width(text)
}

impl Utilities {
    fn save(&self) {
        if let Err(e) = storage::set(STORAGE_KEY, &self.config.encode()) {
            warn(&format!("settings not saved: {e}"));
        }
    }

    /// Persist before replacing the in-memory model. A full 4KiB storage
    /// record must never give the user a false 'saved' confirmation.
    fn commit_waypoints(&mut self, updated: WaypointStore) -> Result<(), String> {
        let bytes = updated.encode();
        if bytes.len() > 4096 {
            return Err("Waypoints non sauvegardés : limite de stockage 4 Ko".into());
        }
        storage::set(extras::STORAGE_KEY, &bytes)
            .map_err(|e| format!("Sauvegarde impossible : {e}"))?;
        self.waypoints = updated;
        Ok(())
    }

    fn push_settings(&self) {
        let page = settings::widgets(&self.config, &self.provider.features());
        ui::set_settings(&page);
    }

    fn rebuild_armor(&mut self) {
        let pieces = self
            .equipment
            .as_deref()
            .map(armor::pieces)
            .unwrap_or_default();
        self.armor_shapes = hud::armor_panel(&self.config.armor, &pieces, &width_of)
            .map(|p| hud::place(&p, self.gui))
            .unwrap_or_default();
    }

    fn rebuild_pickups(&mut self) {
        let rows = self.notifier.rows();
        self.pickup_shapes = hud::pickup_panel(&self.config.pickup, &rows)
            .map(|p| hud::place(&p, self.gui))
            .unwrap_or_default();
    }

    fn push_hud(&mut self) {
        let all: Vec<Shape> = self
            .armor_shapes
            .iter()
            .chain(self.pickup_shapes.iter())
            .chain(self.extras_shapes.iter())
            .cloned()
            .collect();
        if all.is_empty() && !self.hud_sent {
            return;
        }
        render::set_hud(&all);
        self.hud_sent = !all.is_empty();
    }

    /// After any change of the settings: persist, refresh the page, redraw.
    fn changed(&mut self) {
        self.save();
        self.push_settings();
        self.notifier.configure(
            self.config.pickup.max_entries(),
            self.config.pickup.lifetime(),
        );
        self.nutrition_drawn = None;
        self.rebuild_armor();
        self.rebuild_pickups();
        self.telemetry.minimap(&self.config.extras);
        self.push_hud();
    }

    fn set_enabled(&mut self, on: bool) {
        self.config.nutrition.enabled = on;
        self.changed();
    }

    /// No player (title screen, between worlds): nothing shown, nothing kept.
    fn clear_all(&mut self) {
        if self.nutrition_drawn.take().is_some() {
            render::clear_corner();
        }
        self.equipment = None;
        self.armor_shapes.clear();
        self.pickup_shapes.clear();
        self.extras_shapes.clear();
        self.telemetry.clear();
        self.notifier.clear();
        // A world is no longer active: do not select the last profile in
        // another world that happens to use the same dimension name.
        self.waypoints.active = None;
        self.push_hud();
    }

    fn handle_input(&mut self) {
        for event in input::poll() {
            match event {
                input::Event::KeyDown(name) if name == KEY_TOGGLE => {
                    let on = !self.config.nutrition.enabled;
                    self.set_enabled(on);
                }
                _ => {}
            }
        }
    }

    fn refresh_nutrition(&mut self, vitals: &provider::Vitals) {
        let held = self.provider.held_food();
        let snapshot = nutrition::snapshot(vitals, held.as_ref().map(|(id, f)| (id.as_str(), *f)));
        let key = (snapshot, self.config.nutrition.clone());
        if self.nutrition_drawn.as_ref() == Some(&key) {
            return;
        }
        match hud::nutrition_panel(&key.1, &key.0, &width_of) {
            Some(panel) => {
                render::set_corner(panel.corner, panel.width, panel.height, &panel.shapes)
            }
            None => render::clear_corner(),
        }
        self.nutrition_drawn = Some(key);
    }

    fn refresh_armor(&mut self) {
        if !self.provider.features().equipment {
            return;
        }
        let now = self.provider.equipment();
        if now != self.equipment {
            self.equipment = now;
            self.rebuild_armor();
            self.push_hud();
        }
    }

    fn poll_pickups(&mut self) {
        if !self.provider.features().pickups {
            return;
        }
        // Always drained, shown only when the module is on.
        for p in self.provider.pickups() {
            if self.config.pickup.enabled {
                self.notifier.push(&p.item, p.count, &width_of);
            }
        }
    }

    fn refresh_extras(&mut self, tick: u64, player: &game::Player) {
        self.telemetry.sample(
            input::held(),
            tick,
            self.config.extras.enabled && self.config.extras.held_changes,
        );
        self.telemetry.minimap(&self.config.extras);
        let lines = self
            .telemetry
            .lines(&self.config.extras, &self.waypoints, player, tick);
        if self.telemetry.has_panel_changed(&lines, self.gui) {
            self.extras_shapes = self.telemetry.panel(&lines, self.gui);
            self.push_hud();
        }
    }

    fn profile_command(&mut self, args: &[&str]) -> Result<Option<String>, String> {
        match args {
            ["list"] => Ok(Some(if self.waypoints.profiles.is_empty() {
                "Aucun profil. /utilities profile add NOM".into()
            } else {
                self.waypoints
                    .profiles
                    .iter()
                    .map(|p| {
                        format!(
                            "{}{}",
                            if self.waypoints.active.as_deref() == Some(p.name.as_str()) {
                                "*"
                            } else {
                                ""
                            },
                            p.name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ; ")
            })),
            ["add", name] => {
                let mut updated = self.waypoints.clone();
                updated.create_profile(name)?;
                self.commit_waypoints(updated)?;
                Ok(Some(format!(
                    "Profil {name} créé ; /utilities profile use {name}"
                )))
            }
            ["use", name] => {
                // A selected profile is intentionally session-only.
                self.waypoints.use_profile(name)?;
                Ok(Some(format!(
                    "Profil {name} sélectionné pour cette session"
                )))
            }
            ["off"] => {
                self.waypoints.active = None;
                Ok(Some("Profil de navigation désactivé".into()))
            }
            ["remove", name] => {
                let mut updated = self.waypoints.clone();
                if !updated.remove_profile(name) {
                    return Err("Profil inconnu".into());
                }
                self.commit_waypoints(updated)?;
                Ok(Some(format!("Profil {name} supprimé")))
            }
            _ => Err("Usage : /utilities profile [list|add NOM|use NOM|off|remove NOM]".into()),
        }
    }

    fn waypoint_command(&mut self, args: &[&str]) -> Result<Option<String>, String> {
        match args {
            ["list"] => {
                let profile = self
                    .waypoints
                    .current()
                    .ok_or("Sélectionne d'abord un profil")?;
                Ok(Some(if profile.points.is_empty() {
                    "Aucun waypoint".into()
                } else {
                    profile
                        .points
                        .iter()
                        .map(|w| {
                            format!(
                                "{}{} ({:.0}, {:.0}, {:.0}, {})",
                                if profile.selected.as_deref() == Some(w.name.as_str()) {
                                    "*"
                                } else {
                                    ""
                                },
                                w.name,
                                w.x,
                                w.y,
                                w.z,
                                w.dimension
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ; ")
                }))
            }
            ["add", name] => {
                let p = game::get_player().ok_or("Il faut être en jeu")?;
                let mut updated = self.waypoints.clone();
                updated.add(Waypoint::new(name, &p.dimension, p.x, p.y, p.z)?)?;
                updated.select(name)?;
                self.commit_waypoints(updated)?;
                Ok(Some(format!("Waypoint {name} enregistré")))
            }
            ["select", name] => {
                let mut updated = self.waypoints.clone();
                updated.select(name)?;
                self.commit_waypoints(updated)?;
                Ok(Some(format!("Waypoint {name} sélectionné")))
            }
            ["remove", name] => {
                let mut updated = self.waypoints.clone();
                if !updated.remove(name)? {
                    return Err("Waypoint inconnu".into());
                }
                self.commit_waypoints(updated)?;
                Ok(Some(format!("Waypoint {name} supprimé")))
            }
            ["off"] => {
                let mut updated = self.waypoints.clone();
                updated.current_mut()?.selected = None;
                self.commit_waypoints(updated)?;
                Ok(Some("Cap masqué".into()))
            }
            _ => Err("Usage : /utilities wp [list|add NOM|select NOM|remove NOM|off]".into()),
        }
    }

    fn status(&self) -> String {
        let state = if self.config.nutrition.enabled {
            "activé"
        } else {
            "désactivé"
        };
        let gaps: Vec<&str> = unavailable::gaps(&self.provider.features())
            .iter()
            .map(|g| g.feature)
            .collect();
        let gaps = if gaps.is_empty() {
            "aucune".to_string()
        } else {
            gaps.join(", ")
        };
        format!(
            "Mode {}. Nutrition HUD {state}. Navigation/profils et mini-carte disponibles. Indisponible : {gaps}.",
            self.provider.mode().label()
        )
    }
}

impl Mod for Utilities {
    fn init() -> Result<(Self, Registration), String> {
        let config = storage::get(STORAGE_KEY)
            .map(|bytes| Config::decode(&bytes))
            .unwrap_or_default();
        if let Err(e) = input::register_key(
            KEY_TOGGLE,
            "Afficher/masquer le HUD nutrition",
            "key.keyboard.n",
        ) {
            warn(&format!("key not registered: {e}"));
        }
        let provider = provider();
        let waypoints = if let Some(data) = storage::get(extras::STORAGE_KEY) {
            WaypointStore::decode(&data)
        } else if let Some(data) = storage::get(extras::LEGACY_KEY) {
            WaypointStore::migrate_v1(&data)
        } else {
            WaypointStore::default()
        };
        let features = provider.features();
        let state = Utilities {
            notifier: pickup::Notifier::new(config.pickup.max_entries(), config.pickup.lifetime()),
            config,
            provider,
            nutrition_drawn: None,
            equipment: None,
            armor_shapes: Vec::new(),
            pickup_shapes: Vec::new(),
            extras_shapes: Vec::new(),
            telemetry: Telemetry::default(),
            waypoints,
            hud_sent: false,
            gui: (320.0, 180.0),
        };
        state.push_settings();
        info(&format!(
            "Horizon Utilities ready ({} mode)",
            state.provider.mode().label()
        ));
        let mut hooks = Registration::new().command("utilities").on_tick();
        if features.pickups {
            hooks = hooks.on_frame();
        }
        Ok((state, hooks))
    }

    fn tick(&mut self, tick: u64) {
        self.handle_input();
        let Some(vitals) = self.provider.vitals() else {
            self.clear_all();
            return;
        };
        let gui = render::gui_size();
        if gui != self.gui {
            self.gui = gui;
            self.rebuild_armor();
            self.rebuild_pickups();
            self.push_hud();
        }
        self.refresh_nutrition(&vitals);
        self.refresh_armor();
        self.poll_pickups();
        if let Some(player) = game::get_player() {
            self.refresh_extras(tick, &player);
        }
    }

    fn frame(&mut self, seconds: f32) {
        if self.notifier.is_empty() {
            return;
        }
        self.notifier.advance(seconds);
        self.rebuild_pickups();
        self.push_hud();
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let mut words = args.split_whitespace();
        let first = words.next();
        if first == Some("profile") {
            return self.profile_command(&words.collect::<Vec<_>>());
        }
        if first == Some("wp") {
            return self.waypoint_command(&words.collect::<Vec<_>>());
        }
        match (first, words.next()) {
            (None, _) | (Some("status"), _) => Ok(Some(self.status())),
            (Some("nutrition"), Some("on")) => {
                self.set_enabled(true);
                Ok(Some("Nutrition HUD activé".into()))
            }
            (Some("nutrition"), Some("off")) => {
                self.set_enabled(false);
                Ok(Some("Nutrition HUD désactivé".into()))
            }
            (Some("nutrition"), Some("toggle")) => {
                let on = !self.config.nutrition.enabled;
                self.set_enabled(on);
                Ok(Some(self.status()))
            }
            (Some("extras"), Some("on")) => {
                self.config.extras.enabled = true;
                self.changed();
                Ok(Some("Utilitaires activés".into()))
            }
            (Some("extras"), Some("off")) => {
                self.config.extras.enabled = false;
                self.changed();
                Ok(Some("Utilitaires désactivés".into()))
            }
            (Some("sort"), region) => {
                let region = match region {
                    None | Some("main") => SortRegion::Main,
                    Some("container") => SortRegion::Container,
                    Some(_) => return Err("Usage : /utilities sort [main|container]".into()),
                };
                self.provider.sort(region).map(|()| Some("Tri demandé".into()))
            }
            (Some("reset"), _) => {
                self.config = Config::default();
                self.changed();
                Ok(Some("Réglages réinitialisés".into()))
            }
            _ => Err(
                "Usage : /utilities [status | nutrition on|off|toggle | extras on|off | profile ... | wp ... | sort [main|container] | reset]"
                    .into(),
            ),
        }
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        if settings::apply(&mut self.config, id) {
            self.changed();
        }
    }

    fn ui_settings_opened(&mut self) {
        self.push_settings();
    }
}

horizon_mod_sdk::export_mod!(Utilities);
