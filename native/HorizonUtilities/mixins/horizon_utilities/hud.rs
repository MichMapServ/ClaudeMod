fn horizon_utilities_hud(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, corners: &mut horizon_client::extras_hud::Corners) {
    crate::game::horizon_utilities_native::draw(game, batch, corners);
}
