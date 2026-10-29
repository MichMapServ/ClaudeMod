fn horizon_utilities_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if (command == "utilities" || command.starts_with("utilities ")) && game.native_on(crate::game::horizon_utilities_native::ID) {
        *ran = true;
        crate::game::horizon_utilities_native::command(game, command);
    }
}
