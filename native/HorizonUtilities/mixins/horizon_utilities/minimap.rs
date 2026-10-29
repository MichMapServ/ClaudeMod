fn horizon_utilities_minimap(game: &crate::game::Game<'_>, minimap: &mut Option<crate::game::native_hooks::NativeMinimap>) {
    crate::game::horizon_utilities_native::minimap(game, minimap);
}
