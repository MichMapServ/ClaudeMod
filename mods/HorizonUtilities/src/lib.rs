//! Horizon Utilities, entirely in a Horizon .hmod (no companion).
//! Only use interfaces exposed to WASM by the official SDK.
mod config;
mod extras;
mod food;
mod hud;
mod nutrition;
mod settings;
mod unavailable;

use config::{Config, NutritionConfig, STORAGE_KEY};
use extras::{Runtime as ExtrasRuntime, Waypoint, Waypoints, WAYPOINTS_KEY};
use horizon_mod_sdk::{Mod, Registration, game, info, input, render, storage, ui, warn};
use nutrition::Snapshot;

const KEY_TOGGLE: &str = "toggle_nutrition";

struct Utilities {
    config: Config,
    waypoints: Waypoints,
    extras: ExtrasRuntime,
    /// Unchanged nutrition panels are not re-submitted.
    drawn: Option<(Snapshot, NutritionConfig)>,
}

impl Utilities {
    fn save(&self) {
        if let Err(e) = storage::set(STORAGE_KEY, &self.config.encode()) {
            warn(&format!("settings not saved: {e}"));
        }
    }

    fn save_waypoints(&self) {
        if let Err(e) = storage::set(WAYPOINTS_KEY, &self.waypoints.encode()) {
            warn(&format!("waypoints not saved: {e}"));
        }
    }

    fn push_settings(&self) {
        ui::set_settings(&settings::widgets(&self.config));
    }

    fn changed(&mut self) {
        self.save();
        self.push_settings();
        self.drawn = None;
    }

    fn set_enabled(&mut self, on: bool) {
        self.config.nutrition.enabled = on;
        self.changed();
    }

    fn clear(&mut self) {
        if self.drawn.take().is_some() {
            render::clear_corner();
        }
        self.extras.clear();
    }

    fn handle_input(&mut self) {
        for event in input::poll() {
            if let input::Event::KeyDown(name) = event {
                if name == KEY_TOGGLE {
                    self.set_enabled(!self.config.nutrition.enabled);
                }
            }
        }
    }

    fn refresh(&mut self, tick: u64) {
        let Some(player) = game::get_player() else {
            self.clear();
            return;
        };
        let held = input::held();
        let nutrition = nutrition::snapshot(player.food, held.as_ref().map(|h| h.id.as_str()));

        self.extras.hand(
            held,
            tick,
            self.config.extras.enabled && self.config.extras.held_changes,
        );
        self.extras
            .refresh(&self.config.extras, &self.waypoints, &player, tick);

        let key = (nutrition, self.config.nutrition.clone());
        if self.drawn.as_ref() == Some(&key) {
            return;
        }
        match hud::build(&key.1, &key.0, &render::text_width) {
            Some(panel) => {
                render::set_corner(panel.corner, panel.width, panel.height, &panel.shapes)
            }
            None => render::clear_corner(),
        }
        self.drawn = Some(key);
    }

    fn status(&self) -> String {
        let gaps: Vec<&str> = unavailable::GAPS.iter().map(|g| g.feature).collect();
        format!(
            "Nutrition : {}. Modules natifs : {}. {} waypoint(s). API absente : {}.",
            if self.config.nutrition.enabled { "ON" } else { "OFF" },
            if self.config.extras.enabled { "ON" } else { "OFF" },
            self.waypoints.points.len(),
            gaps.join(", ")
        )
    }

    fn waypoint_command(&mut self, args: &[&str]) -> Result<Option<String>, String> {
        match args {
            ["list"] => Ok(Some(self.waypoints.list())),
            ["add", name] => {
                let player = game::get_player().ok_or("Il faut être en jeu")?;
                let waypoint = Waypoint::from_player(name, &player)?;
                self.waypoints.add(waypoint)?;
                self.waypoints.select(name)?;
                self.save_waypoints();
                Ok(Some(format!("Waypoint {name} créé et sélectionné")))
            }
            ["select", name] => {
                self.waypoints.select(name)?;
                self.save_waypoints();
                Ok(Some(format!("Waypoint {name} sélectionné")))
            }
            ["remove", name] => {
                if !self.waypoints.remove(name) {
                    return Err(format!("Waypoint inconnu : {name}"));
                }
                self.save_waypoints();
                Ok(Some(format!("Waypoint {name} supprimé")))
            }
            ["off"] => {
                self.waypoints.active = None;
                self.save_waypoints();
                Ok(Some("Navigation suspendue".into()))
            }
            ["clear"] => {
                self.waypoints = Waypoints::default();
                self.save_waypoints();
                Ok(Some("Waypoints supprimés".into()))
            }
            _ => Err("Usage : /utilities wp [add NOM | select NOM | remove NOM | list | off | clear]".into()),
        }
    }
}

impl Mod for Utilities {
    fn init() -> Result<(Self, Registration), String> {
        let config = storage::get(STORAGE_KEY)
            .map(|bytes| Config::decode(&bytes))
            .unwrap_or_default();
        let waypoints = storage::get(WAYPOINTS_KEY)
            .map(|bytes| Waypoints::decode(&bytes))
            .unwrap_or_default();
        if let Err(e) = input::register_key(
            KEY_TOGGLE,
            "Afficher/masquer le HUD nutrition",
            "key.keyboard.n",
        ) {
            warn(&format!("key not registered: {e}"));
        }
        let state = Utilities {
            config,
            waypoints,
            extras: ExtrasRuntime::default(),
            drawn: None,
        };
        state.push_settings();
        info("Horizon Utilities: HUD nutrition, navigation et diagnostics disponibles");
        Ok((state, Registration::new().command("utilities").on_tick()))
    }

    fn tick(&mut self, tick: u64) {
        self.handle_input();
        self.refresh(tick);
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let words: Vec<&str> = args.split_whitespace().collect();
        match words.as_slice() {
            [] | ["status"] => Ok(Some(self.status())),
            ["nutrition", "on"] => {
                self.set_enabled(true);
                Ok(Some("Nutrition HUD activé".into()))
            }
            ["nutrition", "off"] => {
                self.set_enabled(false);
                Ok(Some("Nutrition HUD désactivé".into()))
            }
            ["nutrition", "toggle"] => {
                self.set_enabled(!self.config.nutrition.enabled);
                Ok(Some(self.status()))
            }
            ["extras", "on"] => {
                self.config.extras.enabled = true;
                self.changed();
                Ok(Some("Modules natifs activés".into()))
            }
            ["extras", "off"] => {
                self.config.extras.enabled = false;
                self.changed();
                Ok(Some("Modules natifs désactivés".into()))
            }
            ["extras", "toggle"] => {
                self.config.extras.enabled = !self.config.extras.enabled;
                self.changed();
                Ok(Some(self.status()))
            }
            ["wp", rest @ ..] => self.waypoint_command(rest),
            ["reset"] => {
                self.config = Config::default();
                self.changed();
                Ok(Some("Réglages réinitialisés (waypoints conservés)".into()))
            }
            _ => Err("Usage : /utilities [status | nutrition on|off|toggle | extras on|off|toggle | wp ... | reset]".into()),
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
