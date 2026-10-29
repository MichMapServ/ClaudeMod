fn status_effect_atelier_hud(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    inventory_effects: bool,
) {
    crate::game::status_effect_atelier::hud(game, batch, inventory_effects);
}
