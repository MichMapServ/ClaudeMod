fn horizon_utilities_settings_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::horizon_utilities_native::ID {
        crate::game::horizon_utilities_native::settings_click(game, button);
    }
}
