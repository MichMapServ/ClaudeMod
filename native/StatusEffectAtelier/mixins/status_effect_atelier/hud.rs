fn status_effect_atelier_hud(
    game: &crate::game::Game<'_>,
    inventory_effects: bool,
    batch: &mut horizon_client::hud::HudBatch,
) {
    crate::game::status_effect_atelier::hud(game, batch, inventory_effects);
}
