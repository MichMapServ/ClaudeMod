fn status_effect_atelier_settings_click(
    game: &mut crate::game::Game<'_>, mod_id: &str, button: usize,
) {
    if mod_id == crate::game::status_effect_atelier::ID {
        crate::game::status_effect_atelier::settings_click(game, button);
    }
}
