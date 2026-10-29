//! Mode étendu: the capabilities `vitals`, `inventory` and `pickups` of a
//! Horizon extended as `ENGINE_PATCH.md` says. Compiled only with the feature
//! `native`, against the extended SDK: these interfaces do not exist in the
//! official one.
use crate::provider::{Features, FoodInfo, Mode, Pickup, Provider, SortRegion, Stack, Vitals};
use horizon_mod_sdk::{inventory, pickups, vitals};

pub struct NativeProvider;

impl Provider for NativeProvider {
    fn mode(&self) -> Mode {
        Mode::Native
    }

    fn features(&self) -> Features {
        Features {
            saturation: true,
            equipment: true,
            pickups: true,
            sort: true,
        }
    }

    fn vitals(&mut self) -> Option<Vitals> {
        vitals::get().map(|v| Vitals {
            food: v.food,
            saturation: Some(v.saturation),
            exhaustion: v.exhaustion,
        })
    }

    fn held_food(&mut self) -> Option<(String, FoodInfo)> {
        vitals::held_food().map(|f| {
            let info = FoodInfo {
                nutrition: f.nutrition,
                saturation: Some(f.saturation),
            };
            (f.item, info)
        })
    }

    fn equipment(&mut self) -> Option<Vec<Option<Stack>>> {
        let slots = inventory::equipment()
            .into_iter()
            .map(|slot| {
                slot.map(|s| Stack {
                    item: s.item,
                    count: s.count,
                    damage: s.damage,
                    max_damage: s.max_damage,
                })
            })
            .collect();
        Some(slots)
    }

    fn pickups(&mut self) -> Vec<Pickup> {
        pickups::poll()
            .into_iter()
            .map(|p| Pickup {
                item: p.item,
                count: p.count,
            })
            .collect()
    }

    fn sort(&mut self, region: SortRegion) -> Result<(), String> {
        let region = match region {
            SortRegion::Main => inventory::Region::Main,
            SortRegion::Container => inventory::Region::Container,
        };
        inventory::sort(region)
    }
}
