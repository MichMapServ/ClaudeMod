//! The mod's page in Options > Mods (capability `ui`).
//!
//! Every button is a real setting: a click comes back as `ui_select(id, 0)`
//! and [`apply`] changes the config. The page also lists, as plain text, what
//! the mod cannot do yet, so no dead button stands for a missing feature.

use crate::config::Config;
use crate::unavailable::GAPS;
use horizon_mod_sdk::ui::{ButtonWidget, TextWidget, Widget};

pub const TOGGLE: u32 = 10;
pub const SHOW_BAR: u32 = 11;
pub const SHOW_PREVIEW: u32 = 12;
pub const SHOW_HELD: u32 = 13;
pub const CORNER: u32 = 14;
pub const SCALE: u32 = 15;
pub const OPACITY: u32 = 16;
pub const RESET: u32 = 99;

/// Applies a click on button `id`; false for an id that is not a setting.
pub fn apply(config: &mut Config, id: u32) -> bool {
    let n = &mut config.nutrition;
    match id {
        TOGGLE => n.enabled = !n.enabled,
        SHOW_BAR => n.show_bar = !n.show_bar,
        SHOW_PREVIEW => n.show_preview = !n.show_preview,
        SHOW_HELD => n.show_held = !n.show_held,
        CORNER => n.corner = n.corner.next(),
        SCALE => n.cycle_scale(),
        OPACITY => n.cycle_opacity(),
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

pub fn widgets(config: &Config) -> Vec<Widget> {
    let n = &config.nutrition;
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
        button(RESET, "Réinitialiser les réglages".to_string()),
        line(2, "Indisponible (API Horizon 0.1) :", 0xFFAA55),
    ];
    for (i, gap) in GAPS.iter().enumerate() {
        let text = format!("- {} : {}", gap.feature, gap.missing);
        out.push(line(100 + i as u32, &text, 0xAAAAAA));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_button_changes_something() {
        for id in [TOGGLE, SHOW_BAR, SHOW_PREVIEW, SHOW_HELD, CORNER, SCALE, OPACITY] {
            let mut config = Config::default();
            assert!(apply(&mut config, id), "{id}");
            assert_ne!(config, Config::default(), "{id}");
        }
    }

    #[test]
    fn reset_restores_the_defaults() {
        let mut config = Config::default();
        apply(&mut config, TOGGLE);
        apply(&mut config, CORNER);
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
    fn the_page_fits_the_api_limits() {
        let page = widgets(&Config::default());
        assert!(page.len() <= 64);
    }
}
