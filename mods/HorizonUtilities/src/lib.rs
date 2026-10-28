//! Horizon Utilities: a nutrition HUD for Horizon (mod API 0.1).
//!
//! Only what the SDK really offers is implemented. The Inventory Manager,
//! Armor HUD and Pickup Notifier modules need APIs that do not exist yet:
//! see `unavailable.rs` and the README.
mod config;
mod food;
mod hud;
mod nutrition;
mod settings;
mod unavailable;

use config::{Config, NutritionConfig, STORAGE_KEY};
use horizon_mod_sdk::{Mod, Registration, game, info, input, render, storage, ui, warn};
use nutrition::Snapshot;

/// Name of the key `input::register_key` hands back in `key-down`.
const KEY_TOGGLE: &str = "toggle_nutrition";

struct Utilities {
    config: Config,
    /// What the corner panel last showed, so an unchanged panel costs no call.
    drawn: Option<(Snapshot, NutritionConfig)>,
}

impl Utilities {
    fn save(&self) {
        if let Err(e) = storage::set(STORAGE_KEY, &self.config.encode()) {
            warn(&format!("settings not saved: {e}"));
        }
    }

    fn push_settings(&self) {
        ui::set_settings(&settings::widgets(&self.config));
    }

    /// After any change of the settings: persist, refresh the page, redraw.
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

    fn refresh(&mut self) {
        // No player on the title screen: nothing to show, nothing kept.
        let Some(player) = game::get_player() else {
            self.clear();
            return;
        };
        let held = input::held();
        let snapshot = nutrition::snapshot(player.food, held.as_ref().map(|h| h.id.as_str()));
        let key = (snapshot, self.config.nutrition.clone());
        if self.drawn.as_ref() == Some(&key) {
            return;
        }
        let width_of = |text: &str| render::text_width(text);
        match hud::build(&key.1, &key.0, &width_of) {
            Some(panel) => {
                render::set_corner(panel.corner, panel.width, panel.height, &panel.shapes)
            }
            None => render::clear_corner(),
        }
        self.drawn = Some(key);
    }

    fn status(&self) -> String {
        let state = if self.config.nutrition.enabled {
            "activé"
        } else {
            "désactivé"
        };
        let gaps: Vec<&str> = unavailable::GAPS.iter().map(|g| g.feature).collect();
        format!(
            "Nutrition HUD {state}. Indisponible (API absente) : {}.",
            gaps.join(", ")
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
        let state = Utilities {
            config,
            drawn: None,
        };
        state.push_settings();
        info("Horizon Utilities ready");
        Ok((state, Registration::new().command("utilities").on_tick()))
    }

    fn tick(&mut self, _tick: u64) {
        self.handle_input();
        self.refresh();
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let mut words = args.split_whitespace();
        match (words.next(), words.next()) {
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
            (Some("reset"), _) => {
                self.config = Config::default();
                self.changed();
                Ok(Some("Réglages réinitialisés".into()))
            }
            _ => Err("Usage : /utilities [status | nutrition on|off|toggle | reset]".into()),
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
