fn status_effect_atelier_hud(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
) -> ::core::ops::ControlFlow<()> {
    if !crate::game::status_effect_atelier::enabled(game) {
        return ::core::ops::ControlFlow::Continue(());
    }
    crate::game::status_effect_atelier::hud(game, batch);
    ::core::ops::ControlFlow::Break(())
}
