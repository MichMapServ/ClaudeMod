fn status_effect_atelier_settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    if mod_id == crate::game::status_effect_atelier::ID {
        buttons.extend(crate::game::status_effect_atelier::settings(game));
    }
}
