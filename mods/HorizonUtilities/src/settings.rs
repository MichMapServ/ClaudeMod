//! The mod's page in Options > Mods (capability `ui`).
//!
//! Every button is a real setting: a click comes back as `ui_select(id, 0)`
//! and [`apply`] changes the config. A module the current mode cannot serve
//! gets a line of text, never a button.

use crate::config::Config;
use crate::provider::Features;
use crate::unavailable;
use horizon_mod_sdk::ui::{ButtonWidget, TextWidget, Widget};

pub const TOGGLE: u32 = 10;
pub const SHOW_BAR: u32 = 11;
pub const SHOW_PREVIEW: u32 = 12;
pub const SHOW_HELD: u32 = 13;
pub const CORNER: u32 = 14;
pub const SCALE: u32 = 15;
pub const OPACITY: u32 = 16;
pub const ARMOR_TOGGLE: u32 = 20;
pub const ARMOR_CORNER: u32 = 21;
pub const ARMOR_SCALE: u32 = 22;
pub const ARMOR_PERCENT: u32 = 23;
pub const PICKUP_TOGGLE: u32 = 30;
pub const PICKUP_CORNER: u32 = 31;
pub const PICKUP_MAX: u32 = 32;
pub const PICKUP_SECONDS: u32 = 33;
pub const EXTRAS: u32 = 40;
pub const PERFORMANCE: u32 = 41;
pub const SURVIVAL: u32 = 42;
pub const HELD: u32 = 43;
pub const NAVIGATION: u32 = 44;
pub const MINIMAP: u32 = 45;
pub const ROTATE_MAP: u32 = 46;
pub const RESET: u32 = 99;

/// Applies a click on button `id`; false for an id that is not a setting.
pub fn apply(config: &mut Config, id: u32) -> bool {
    match id {
        TOGGLE => config.nutrition.enabled = !config.nutrition.enabled,
        SHOW_BAR => config.nutrition.show_bar = !config.nutrition.show_bar,
        SHOW_PREVIEW => config.nutrition.show_preview = !config.nutrition.show_preview,
        SHOW_HELD => config.nutrition.show_held = !config.nutrition.show_held,
        CORNER => config.nutrition.corner = config.nutrition.corner.next(),
        SCALE => config.nutrition.cycle_scale(),
        OPACITY => config.nutrition.cycle_opacity(),
        ARMOR_TOGGLE => config.armor.enabled = !config.armor.enabled,
        ARMOR_CORNER => config.armor.corner = config.armor.corner.next(),
        ARMOR_SCALE => config.armor.cycle_scale(),
        ARMOR_PERCENT => config.armor.show_percent = !config.armor.show_percent,
        PICKUP_TOGGLE => config.pickup.enabled = !config.pickup.enabled,
        PICKUP_CORNER => config.pickup.corner = config.pickup.corner.next(),
        PICKUP_MAX => config.pickup.cycle_max(),
        PICKUP_SECONDS => config.pickup.cycle_seconds(),
        EXTRAS => config.extras.enabled = !config.extras.enabled,
        PERFORMANCE => config.extras.performance = !config.extras.performance,
        SURVIVAL => config.extras.survival_alerts = !config.extras.survival_alerts,
        HELD => config.extras.held_changes = !config.extras.held_changes,
        NAVIGATION => config.extras.navigation = !config.extras.navigation,
        MINIMAP => config.extras.minimap = !config.extras.minimap,
        ROTATE_MAP => config.extras.rotate_map = !config.extras.rotate_map,
        RESET => *config = Config::default(),
        _ => return false,
    }
    true
}

fn on_off(on: bool) -> &'static str {
    if on { "activé" } else { "désactivé" }
}

fn line(id: u32, text: &str, color: u32) -> Widget {
    Widget::Text(TextWidget {
        id,
        text: text.to_string(),
        color,
    })
}

fn button(id: u32, label: String) -> Widget {
    Widget::Button(ButtonWidget { id, label })
}

pub fn widgets(config: &Config, features: &Features) -> Vec<Widget> {
    let (n, a, p) = (&config.nutrition, &config.armor, &config.pickup);
    let mut out = vec![
        line(1, "Horizon Utilities - Nutrition HUD", 0),
        button(TOGGLE, format!("HUD nutrition : {}", on_off(n.enabled))),
        button(SHOW_BAR, format!("Barre de faim : {}", on_off(n.show_bar))),
        button(
            SHOW_PREVIEW,
            format!("Aperçu des points : {}", on_off(n.show_preview)),
        ),
        button(
            SHOW_HELD,
            format!("Aliment en main : {}", on_off(n.show_held)),
        ),
        button(CORNER, format!("Coin : {}", n.corner.label())),
        button(
            SCALE,
            format!("Échelle de la barre : x{}", n.scale_factor()),
        ),
        button(OPACITY, format!("Opacité : {} %", n.opacity_percent())),
    ];
    if features.equipment {
        out.push(line(3, "Armor HUD", 0));
        out.push(button(
            ARMOR_TOGGLE,
            format!("Armor HUD : {}", on_off(a.enabled)),
        ));
        out.push(button(ARMOR_CORNER, format!("Coin : {}", a.corner.label())));
        out.push(button(
            ARMOR_SCALE,
            format!("Échelle : x{}", a.scale_factor()),
        ));
        out.push(button(
            ARMOR_PERCENT,
            format!("Pourcentage : {}", on_off(a.show_percent)),
        ));
    }
    if features.pickups {
        out.push(line(4, "Pickup Notifier", 0));
        out.push(button(
            PICKUP_TOGGLE,
            format!("Notifications : {}", on_off(p.enabled)),
        ));
        out.push(button(
            PICKUP_CORNER,
            format!("Coin : {}", p.corner.label()),
        ));
        out.push(button(
            PICKUP_MAX,
            format!("Maximum affiché : {}", p.max_entries()),
        ));
        out.push(button(
            PICKUP_SECONDS,
            format!("Durée : {} s", p.lifetime()),
        ));
    }
    out.push(line(5, "Utilitaires intégrés au .hmod", 0x66CCFF));
    out.push(button(
        EXTRAS,
        format!("Utilitaires : {}", on_off(config.extras.enabled)),
    ));
    out.push(button(
        PERFORMANCE,
        format!("FPS / CPS : {}", on_off(config.extras.performance)),
    ));
    out.push(button(
        SURVIVAL,
        format!(
            "Alertes de survie : {}",
            on_off(config.extras.survival_alerts)
        ),
    ));
    out.push(button(
        HELD,
        format!(
            "Variations en main (pas pickup) : {}",
            on_off(config.extras.held_changes)
        ),
    ));
    out.push(button(
        NAVIGATION,
        format!("Navigation : {}", on_off(config.extras.navigation)),
    ));
    out.push(button(
        MINIMAP,
        format!("Mini-carte : {}", on_off(config.extras.minimap)),
    ));
    out.push(button(
        ROTATE_MAP,
        format!("Mini-carte orientée : {}", on_off(config.extras.rotate_map)),
    ));
    out.push(line(
        6,
        "Waypoints : /utilities profile add NOM puis use NOM",
        0xAAAAAA,
    ));
    out.push(button(RESET, "Réinitialiser les réglages".to_string()));
    let gaps = unavailable::gaps(features);
    if !gaps.is_empty() {
        out.push(line(2, "Indisponible dans ce mode :", 0xFFAA55));
    }
    for (i, gap) in gaps.iter().enumerate() {
        let text = format!("- {} : {}", gap.feature, gap.missing);
        out.push(line(100 + i as u32, &text, 0xAAAAAA));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [u32; 22] = [
        TOGGLE,
        SHOW_BAR,
        SHOW_PREVIEW,
        SHOW_HELD,
        CORNER,
        SCALE,
        OPACITY,
        ARMOR_TOGGLE,
        ARMOR_CORNER,
        ARMOR_SCALE,
        ARMOR_PERCENT,
        PICKUP_TOGGLE,
        PICKUP_CORNER,
        PICKUP_MAX,
        PICKUP_SECONDS,
        EXTRAS,
        PERFORMANCE,
        SURVIVAL,
        HELD,
        NAVIGATION,
        MINIMAP,
        ROTATE_MAP,
    ];

    #[test]
    fn every_button_changes_something() {
        for id in ALL {
            let mut config = Config::default();
            assert!(apply(&mut config, id), "{id}");
            assert_ne!(config, Config::default(), "{id}");
        }
    }

    #[test]
    fn button_ids_are_distinct() {
        let mut ids = ALL.to_vec();
        ids.push(RESET);
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ALL.len() + 1);
    }

    #[test]
    fn reset_restores_the_defaults() {
        let mut config = Config::default();
        apply(&mut config, TOGGLE);
        apply(&mut config, ARMOR_CORNER);
        apply(&mut config, PICKUP_MAX);
        assert!(apply(&mut config, RESET));
        assert_eq!(config, Config::default());
    }

    #[test]
    fn unknown_ids_are_ignored() {
        let mut config = Config::default();
        assert!(!apply(&mut config, 12345));
        assert_eq!(config, Config::default());
    }

    #[test]
    fn pages_fit_the_api_limits_in_both_modes() {
        let all = Features {
            saturation: true,
            equipment: true,
            pickups: true,
            sort: true,
        };
        for features in [Features::default(), all] {
            let page = widgets(&Config::default(), &features);
            assert!(page.len() <= 64);
        }
    }

    #[test]
    fn standard_mode_has_no_armor_or_pickup_buttons() {
        let page = widgets(&Config::default(), &Features::default());
        let has = |wanted: u32| {
            page.iter()
                .any(|w| matches!(w, Widget::Button(b) if b.id == wanted))
        };
        assert!(!has(ARMOR_TOGGLE) && !has(PICKUP_TOGGLE));
        assert!(has(TOGGLE) && has(RESET));
    }
}
