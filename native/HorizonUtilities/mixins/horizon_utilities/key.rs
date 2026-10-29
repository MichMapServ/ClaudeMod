fn horizon_utilities_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    if pressed { crate::game::horizon_utilities_native::key(game, key); }
}
