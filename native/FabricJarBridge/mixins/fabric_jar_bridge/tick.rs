fn fabric_bridge_tick(game: &mut crate::game::Game<'_>) {
    if !game.native_window() || !game.native_on(crate::game::fabric_jar_bridge::ID) {
        return;
    }
    let position = game.native_player().map(|p| [p.x, p.y, p.z]);
    crate::game::fabric_java_bridge::tick(position);
}
