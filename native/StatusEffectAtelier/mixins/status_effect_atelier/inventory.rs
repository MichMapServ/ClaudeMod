fn status_effect_atelier_inventory(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    origin: (f32, f32),
    image_width: i32,
) -> ::core::ops::ControlFlow<()> {
    if !crate::game::status_effect_atelier::enabled(game) {
        return ::core::ops::ControlFlow::Continue(());
    }
    crate::game::status_effect_atelier::inventory(game, batch, origin, image_width);
    ::core::ops::ControlFlow::Break(())
}
