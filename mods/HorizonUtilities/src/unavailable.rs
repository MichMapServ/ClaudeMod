//! What cannot be done in the current mode, and why. Shown on the settings
//! page and by `/utilities status`, instead of shipping controls that would
//! do nothing.
//!
//! Standard mode: the official SDK (`horizon:mods@0.1.0`, level 7) has no
//! saturation, no equipment or durability, no pickup event and no inventory
//! slot access (`crates/horizon-mod-sdk/wit/horizon.wit`).
use crate::provider::Features;

pub struct Gap {
    pub feature: &'static str,
    pub missing: &'static str,
}

pub fn gaps(features: &Features) -> Vec<Gap> {
    let mut out = Vec::new();
    if !features.saturation {
        out.push(Gap {
            feature: "Saturation",
            missing: "valeur non exposée par l'API standard : rien n'est affiché",
        });
    }
    if !features.equipment {
        out.push(Gap {
            feature: "Armor HUD",
            missing: "aucune API d'équipement ni de durabilité en mode standard",
        });
    }
    if !features.pickups {
        out.push(Gap {
            feature: "Pickup Notifier",
            missing: "aucun événement de ramassage en mode standard",
        });
    }
    if !features.sort {
        out.push(Gap {
            feature: "Inventory Manager",
            missing: "aucune API d'inventaire en mode standard",
        });
    } else {
        out.push(Gap {
            feature: "Verrouillage et déplacement",
            missing: "non implémentés : ils exigent un hook de veto sur les clics (voir ENGINE_PATCH.md)",
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_mode_lists_every_gap() {
        let gaps = gaps(&Features::default());
        assert_eq!(gaps.len(), 4);
        for gap in &gaps {
            assert!(!gap.feature.is_empty() && !gap.missing.is_empty());
        }
    }

    #[test]
    fn extended_mode_only_lists_what_is_still_missing() {
        let all = Features {
            saturation: true,
            equipment: true,
            pickups: true,
            sort: true,
        };
        let gaps = gaps(&all);
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].feature.starts_with("Verrouillage"));
    }
}
