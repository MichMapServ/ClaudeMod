//! What was asked for but cannot be built on the mod API this mod targets.
//!
//! Proof: `crates/horizon-mod-sdk/wit/horizon.wit` of horizon-rust/horizon at
//! commit f66f66c (package `horizon:mods@0.1.0`). `game::player` holds only
//! position, rotation, health, food (0 to 20) and dimension; `input::held` is
//! the main hand only; `recipes::give` is creative mode only. Nothing reads or
//! moves inventory slots, reads worn armor or durability, or reports a pickup.
//! These are listed on the settings page and by `/utilities status` instead
//! of shipping controls that would do nothing.

pub struct Gap {
    pub feature: &'static str,
    pub missing: &'static str,
}

pub const GAPS: [Gap; 4] = [
    Gap {
        feature: "Inventory Manager",
        missing: "aucune API de lecture ni de déplacement des emplacements d'inventaire ou de conteneur",
    },
    Gap {
        feature: "Armor HUD",
        missing: "aucune API d'équipement porté ni de durabilité",
    },
    Gap {
        feature: "Pickup Notifier",
        missing: "aucun événement de ramassage d'objet",
    },
    Gap {
        feature: "Saturation",
        missing: "la valeur n'est pas exposée : rien n'est affiché",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gap_is_explained() {
        for gap in &GAPS {
            assert!(!gap.feature.is_empty() && !gap.missing.is_empty());
        }
    }
}
