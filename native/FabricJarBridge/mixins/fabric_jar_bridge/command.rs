fn fabric_bridge_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if *ran { return; }
    let body = command.trim();
    if body != "fabricbridge" && body != "fabricbridge list" && body != "fabricbridge help" {
        return;
    }
    *ran = true;
    if body == "fabricbridge help" {
        game.native_chat("Fabric Bridge: /fabricbridge list - jars detectes au demarrage");
        game.native_chat("Seules les ressources compatibles sont importees. Java/Fabric API ne s'execute pas.");
        return;
    }
    for line in crate::game::fabric_jar_bridge::status_lines() {
        game.native_chat(&line);
    }
}
