fn horizon_utilities_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == crate::game::horizon_utilities_native::ID {
        buttons.extend(crate::game::horizon_utilities_native::settings(game));
    }
}
