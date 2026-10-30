fn fabric_bridge_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: crate::game::fabric_jar_bridge::ID.into(),
        title: "Fabric Jar Bridge 0.3.0".into(),
        description: "Carry assets/data; opt-in JVM Fabric entrypoints, client ticks and live player positions".into(),
        keys: Vec::new(),
    });
}
