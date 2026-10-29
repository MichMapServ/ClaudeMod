//! Armor wear: pure logic over the equipment the provider reports. The
//! durability is the game's own (`damage` and `max_damage`); nothing is
//! computed from anything else.
use crate::provider::Stack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wear {
    Fine,
    Worn,
    Low,
    Critical,
}

impl Wear {
    /// 0xRRGGBB.
    pub fn color(self) -> u32 {
        match self {
            Wear::Fine => 0x55FF55,
            Wear::Worn => 0xFFFF55,
            Wear::Low => 0xFFAA00,
            Wear::Critical => 0xFF5555,
        }
    }

    /// A piece about to break.
    pub fn warns(self) -> bool {
        matches!(self, Wear::Low | Wear::Critical)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub item: String,
    /// (remaining, maximum); None for an item that cannot be damaged.
    pub durability: Option<(u32, u32)>,
}

impl Piece {
    pub fn fraction(&self) -> Option<f32> {
        let (left, max) = self.durability?;
        Some((left as f32 / max as f32).clamp(0.0, 1.0))
    }

    pub fn percent(&self) -> Option<u32> {
        self.fraction().map(|f| (f * 100.0).round() as u32)
    }

    pub fn wear(&self) -> Option<Wear> {
        let (left, _) = self.durability?;
        let f = self.fraction()?;
        Some(if left <= 1 || f <= 0.10 {
            Wear::Critical
        } else if f <= 0.25 {
            Wear::Low
        } else if f <= 0.50 {
            Wear::Worn
        } else {
            Wear::Fine
        })
    }
}

fn piece(stack: &Stack) -> Piece {
    let durability = (stack.max_damage > 0).then(|| {
        (
            stack.max_damage.saturating_sub(stack.damage),
            stack.max_damage,
        )
    });
    Piece {
        item: stack.item.clone(),
        durability,
    }
}

/// The four armor slots (head, chest, legs, feet) of the equipment list; the
/// off hand is not armor.
pub fn pieces(equipment: &[Option<Stack>]) -> Vec<Option<Piece>> {
    equipment
        .iter()
        .take(4)
        .map(|slot| slot.as_ref().map(piece))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(damage: u32, max: u32) -> Stack {
        Stack {
            item: "minecraft:iron_helmet".into(),
            count: 1,
            damage,
            max_damage: max,
        }
    }

    #[test]
    fn wear_follows_the_remaining_durability() {
        let wear = |damage| piece(&stack(damage, 100)).wear();
        assert_eq!(wear(0), Some(Wear::Fine));
        assert_eq!(wear(50), Some(Wear::Worn));
        assert_eq!(wear(75), Some(Wear::Low));
        assert_eq!(wear(90), Some(Wear::Critical));
        assert_eq!(wear(99), Some(Wear::Critical));
    }

    #[test]
    fn one_point_left_is_critical_whatever_the_maximum() {
        assert_eq!(piece(&stack(362, 363)).wear(), Some(Wear::Critical));
    }

    #[test]
    fn undamageable_items_have_no_bar() {
        let p = piece(&stack(0, 0));
        assert_eq!(p.durability, None);
        assert_eq!(p.percent(), None);
        assert_eq!(p.wear(), None);
    }

    #[test]
    fn damage_beyond_the_maximum_clamps() {
        let p = piece(&stack(500, 100));
        assert_eq!(p.durability, Some((0, 100)));
        assert_eq!(p.percent(), Some(0));
    }

    #[test]
    fn only_the_four_armor_slots_count() {
        let equipment = vec![
            Some(stack(0, 100)),
            None,
            Some(stack(10, 100)),
            None,
            Some(stack(0, 100)),
        ];
        let pieces = pieces(&equipment);
        assert_eq!(pieces.len(), 4);
        assert!(pieces[1].is_none());
        assert_eq!(pieces[2].as_ref().and_then(Piece::percent), Some(90));
    }

    #[test]
    fn warnings_start_at_low() {
        assert!(!Wear::Worn.warns());
        assert!(Wear::Low.warns() && Wear::Critical.warns());
    }
}
