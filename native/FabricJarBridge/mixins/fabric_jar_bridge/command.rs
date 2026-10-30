fn fabric_bridge_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if *ran { return; }
    let body = command.trim();
    if let Some(id) = body.strip_prefix("fabricbridge trust ") {
        *ran = true;
        game.native_chat(&crate::game::fabric_java_bridge::trust(id.trim()));
        return;
    }
    if body == "fabricbridge help" {
        *ran = true;
        game.native_chat("Fabric Bridge: /fabricbridge list | /fabricbridge trust <id>");
        game.native_chat("Trust autorise le bytecode Java du JAR exact (SHA-256) avec vos droits Windows.");
        game.native_chat("Redemarrage requis; Java Fabric API / Minecraft / Mixins non generiques.");
        return;
    }
    if body == "fabricbridge" || body == "fabricbridge list" {
        *ran = true;
        for line in crate::game::fabric_jar_bridge::status_lines() {game.native_chat(&line);}
        return;
    }
    // A real Java ModInitializer can register a Horizon command using
    // dev.horizon.fabricbridge.HorizonBridge.registerCommand(name, reply).
    if let Some(reply) = crate::game::fabric_java_bridge::command(body) {
        *ran = true;
        game.native_chat(&reply);
    }
}
