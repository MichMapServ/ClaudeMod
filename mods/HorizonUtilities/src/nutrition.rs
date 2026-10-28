//! Nutrition logic: pure, no SDK types.
//!
//! The hunger level is the real one from `game::get_player`. The held food's
//! points come from the built-in vanilla table in `food.rs`.

use crate::food;

/// The hunger bar's size.
pub const MAX_FOOD: u32 = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldFood {
    pub item: String,
    pub points: u32,
    pub note: Option<&'static str>,
}

/// What the HUD depends on; compared to skip redundant redraws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub food: u32,
    pub held: Option<HeldFood>,
}

pub fn snapshot(food: u32, held_item: Option<&str>) -> Snapshot {
    let held = held_item.and_then(|id| {
        food::lookup(id).map(|f| HeldFood {
            item: id.to_string(),
            points: f.points,
            note: f.note,
        })
    });
    Snapshot {
        food: food.min(MAX_FOOD),
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

    #[test]
    fn snapshot_keeps_only_known_foods() {
        let s = snapshot(14, Some("minecraft:apple"));
        assert_eq!(s.food, 14);
        assert_eq!(s.held.as_ref().map(|h| h.points), Some(4));
        assert!(snapshot(14, Some("minecraft:stone")).held.is_none());
        assert!(snapshot(14, None).held.is_none());
    }

    #[test]
    fn snapshot_clamps_the_hunger_level() {
        assert_eq!(snapshot(99, None).food, MAX_FOOD);
    }

    #[test]
    fn preview_caps_at_twenty_and_counts_waste() {
        assert_eq!(
            preview(14, 4),
            Preview {
                current: 14,
                after: 18,
                wasted: 0
            }
        );
        assert_eq!(
            preview(18, 8),
            Preview {
                current: 18,
                after: 20,
                wasted: 6
            }
        );
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
