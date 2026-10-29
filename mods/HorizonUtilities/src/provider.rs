//! Where the data comes from.
//!
//! A component's imports are fixed when it is built, so the provider is
//! chosen at build time, not at run time: [`StandardProvider`] (official SDK,
//! the default) or `native::NativeProvider` (feature `native`: a Horizon
//! extended with the capabilities of `ENGINE_PATCH.md`; the game refuses that
//! build on a Horizon without them, with "update Horizon").
use crate::food;
use horizon_mod_sdk::{game, input};

#[derive(Clone, Debug, PartialEq)]
pub struct Vitals {
    pub food: u32,
    /// None when the API does not expose it: never estimated.
    pub saturation: Option<f32>,
    pub exhaustion: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoodInfo {
    pub nutrition: u32,
    pub saturation: Option<f32>,
}

/// An item stack as the equipment reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
    pub item: String,
    pub count: u32,
    pub damage: u32,
    /// 0 for an item that cannot be damaged.
    pub max_damage: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pickup {
    pub item: String,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortRegion {
    Main,
    Container,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Standard,
    Native,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Standard => "standard",
            Mode::Native => "étendu",
        }
    }
}

/// What this provider can really deliver; a feature that is off is never
/// shown as active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Features {
    pub saturation: bool,
    pub equipment: bool,
    pub pickups: bool,
    pub sort: bool,
}

pub trait Provider {
    fn mode(&self) -> Mode;
    fn features(&self) -> Features;
    /// None when no player is in a world.
    fn vitals(&mut self) -> Option<Vitals>;
    /// The main hand's item and its food values, when it is food.
    fn held_food(&mut self) -> Option<(String, FoodInfo)>;
    /// Head, chest, legs, feet, off hand; None when unavailable.
    fn equipment(&mut self) -> Option<Vec<Option<Stack>>> {
        None
    }
    /// Items really picked up since the last call.
    fn pickups(&mut self) -> Vec<Pickup> {
        Vec::new()
    }
    fn sort(&mut self, _region: SortRegion) -> Result<(), String> {
        Err("tri indisponible : aucune API d'inventaire dans ce mode".into())
    }
}

/// Official SDK only: hunger from `game`, the held item from `input`, food
/// points from the built-in vanilla table.
#[cfg_attr(feature = "native", allow(dead_code))]
pub struct StandardProvider;

impl Provider for StandardProvider {
    fn mode(&self) -> Mode {
        Mode::Standard
    }

    fn features(&self) -> Features {
        Features::default()
    }

    fn vitals(&mut self) -> Option<Vitals> {
        game::get_player().map(|p| Vitals {
            food: p.food,
            saturation: None,
            exhaustion: None,
        })
    }

    fn held_food(&mut self) -> Option<(String, FoodInfo)> {
        let held = input::held()?;
        let info = food::lookup(&held.id)?;
        let food = FoodInfo {
            nutrition: info.points,
            saturation: None,
        };
        Some((held.id, food))
    }
}
