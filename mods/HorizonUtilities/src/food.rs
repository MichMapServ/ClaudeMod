//! Built-in table of vanilla foods: hunger points restored by an item.
//!
//! Horizon's mod API does not expose an item's food values, so this is a
//! table of vanilla's, keyed by item id. It is not read from the game: an
//! item missing here (a modded or data-pack food) simply has no preview.
//! Saturation is deliberately absent: neither the player's nor a food's is
//! available from the API, and none is ever displayed.

pub struct Food {
    /// Hunger points restored (the hunger bar is 20 points, 10 icons).
    pub points: u32,
    /// A side effect worth knowing about, if any.
    pub note: Option<&'static str>,
}

/// `item_id` as the API gives it (`minecraft:apple`).
pub fn lookup(item_id: &str) -> Option<Food> {
    let name = item_id.strip_prefix("minecraft:")?;
    let (points, note): (u32, Option<&'static str>) = match name {
        "apple" => (4, None),
        "baked_potato" => (5, None),
        "beef" => (3, None),
        "beetroot" => (1, None),
        "beetroot_soup" => (6, None),
        "bread" => (5, None),
        "carrot" => (3, None),
        "chicken" => (2, Some("Peut donner Faim")),
        "chorus_fruit" => (4, Some("Téléporte")),
        "cod" => (2, None),
        "cooked_beef" => (8, None),
        "cooked_chicken" => (6, None),
        "cooked_cod" => (5, None),
        "cooked_mutton" => (6, None),
        "cooked_porkchop" => (8, None),
        "cooked_rabbit" => (5, None),
        "cooked_salmon" => (6, None),
        "cookie" => (2, None),
        "dried_kelp" => (1, None),
        "enchanted_golden_apple" => (4, Some("Effets puissants")),
        "glow_berries" => (2, None),
        "golden_apple" => (4, Some("Effets")),
        "golden_carrot" => (6, None),
        "honey_bottle" => (6, Some("Retire le poison")),
        "melon_slice" => (2, None),
        "mushroom_stew" => (6, None),
        "mutton" => (2, None),
        "poisonous_potato" => (2, Some("Poison possible")),
        "porkchop" => (3, None),
        "potato" => (1, None),
        "pufferfish" => (1, Some("Poison")),
        "pumpkin_pie" => (8, None),
        "rabbit" => (3, None),
        "rabbit_stew" => (10, None),
        "rotten_flesh" => (4, Some("Faim probable")),
        "salmon" => (2, None),
        "spider_eye" => (2, Some("Poison")),
        "suspicious_stew" => (6, Some("Effet selon la fleur")),
        "sweet_berries" => (2, None),
        "tropical_fish" => (1, None),
        _ => return None,
    };
    Some(Food { points, note })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_common_foods() {
        assert_eq!(lookup("minecraft:apple").map(|f| f.points), Some(4));
        assert_eq!(lookup("minecraft:cooked_beef").map(|f| f.points), Some(8));
        assert_eq!(lookup("minecraft:rabbit_stew").map(|f| f.points), Some(10));
    }

    #[test]
    fn unknown_or_foreign_items_have_no_value() {
        assert!(lookup("minecraft:stone").is_none());
        assert!(lookup("apple").is_none());
        assert!(lookup("othermod:apple").is_none());
        assert!(lookup("").is_none());
    }

    #[test]
    fn no_food_restores_more_than_a_full_bar() {
        for id in [
            "golden_carrot",
            "rabbit_stew",
            "pumpkin_pie",
            "cooked_porkchop",
        ] {
            let f = lookup(&format!("minecraft:{id}")).unwrap();
            assert!((1..=20).contains(&f.points));
        }
    }
}
