//! Nutrition logic: pure, no SDK types.
//!
//! A value the provider does not give (saturation in standard mode) stays
//! `None` all the way to the HUD: nothing is estimated or reconstructed.

use crate::food;
use crate::provider::{FoodInfo, Vitals};

/// The hunger bar's size.
pub const MAX_FOOD: u32 = 20;

#[derive(Clone, Debug, PartialEq)]
pub struct HeldFood {
    pub item: String,
    pub points: u32,
    pub saturation: Option<f32>,
    pub note: Option<&'static str>,
}

/// What the HUD depends on; compared to skip redundant redraws.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub food: u32,
    pub saturation: Option<f32>,
    pub exhaustion: Option<f32>,
    pub held: Option<HeldFood>,
}

/// Finite, in range and rounded to a tenth, so a float flicker is no redraw.
fn clean(v: f32) -> Option<f32> {
    v.is_finite()
        .then(|| (v.clamp(0.0, 1000.0) * 10.0).round() / 10.0)
}

pub fn snapshot(vitals: &Vitals, held: Option<(&str, FoodInfo)>) -> Snapshot {
    let held = held.map(|(id, info)| HeldFood {
        item: id.to_string(),
        points: info.nutrition,
        saturation: info.saturation.and_then(clean),
        note: food::lookup(id).and_then(|f| f.note),
    });
    Snapshot {
        food: vitals.food.min(MAX_FOOD),
        saturation: vitals.saturation.and_then(clean),
        exhaustion: vitals.exhaustion.and_then(clean),
        held,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preview {
    pub current: u32,
    pub after: u32,
    /// Points that would be lost to the 20 cap.
    pub wasted: u32,
}

pub fn preview(food: u32, points: u32) -> Preview {
    let current = food.min(MAX_FOOD);
    let total = current + points;
    let after = total.min(MAX_FOOD);
    Preview {
        current,
        after,
        wasted: total - after,
    }
}

/// How much of icon `unit` (0 to 9) a hunger value fills, in halves (0 to 2).
pub fn half_units(value: u32, unit: u32) -> u32 {
    value.saturating_sub(unit * 2).min(2)
}

/// `minecraft:cooked_beef` as `Cooked beef`.
pub fn display_name(item_id: &str) -> String {
    let path = item_id.split_once(':').map_or(item_id, |(_, p)| p);
    let text = path.replace('_', " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vitals(food: u32) -> Vitals {
        Vitals {
            food,
            saturation: None,
            exhaustion: None,
        }
    }

    fn apple() -> Option<(&'static str, FoodInfo)> {
        let info = FoodInfo {
            nutrition: 4,
            saturation: None,
        };
        Some(("minecraft:apple", info))
    }

    #[test]
    fn snapshot_carries_the_held_food() {
        let s = snapshot(&vitals(14), apple());
        assert_eq!(s.food, 14);
        assert_eq!(s.held.as_ref().map(|h| h.points), Some(4));
        assert!(snapshot(&vitals(14), None).held.is_none());
    }

    #[test]
    fn missing_saturation_stays_missing() {
        let s = snapshot(&vitals(14), apple());
        assert_eq!(s.saturation, None);
        assert_eq!(s.exhaustion, None);
        assert_eq!(s.held.and_then(|h| h.saturation), None);
    }

    #[test]
    fn real_saturation_is_kept_and_rounded() {
        let v = Vitals {
            food: 18,
            saturation: Some(4.4999),
            exhaustion: Some(f32::NAN),
        };
        let s = snapshot(&v, None);
        assert_eq!(s.saturation, Some(4.5));
        assert_eq!(s.exhaustion, None);
    }

    #[test]
    fn snapshot_clamps_the_hunger_level() {
        assert_eq!(snapshot(&vitals(99), None).food, MAX_FOOD);
    }

    #[test]
    fn preview_caps_at_twenty_and_counts_waste() {
        let p = preview(14, 4);
        assert_eq!((p.current, p.after, p.wasted), (14, 18, 0));
        let p = preview(18, 8);
        assert_eq!((p.current, p.after, p.wasted), (18, 20, 6));
        assert_eq!(preview(20, 5).wasted, 5);
    }

    #[test]
    fn icons_fill_in_halves() {
        assert_eq!(half_units(14, 6), 2);
        assert_eq!(half_units(15, 7), 1);
        assert_eq!(half_units(14, 7), 0);
        assert_eq!(half_units(20, 9), 2);
        assert_eq!(half_units(0, 0), 0);
    }

    #[test]
    fn names_are_readable() {
        assert_eq!(display_name("minecraft:cooked_beef"), "Cooked beef");
        assert_eq!(display_name("apple"), "Apple");
        assert_eq!(display_name(""), "");
    }
}
