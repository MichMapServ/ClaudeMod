fn fabric_bridge_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: crate::game::fabric_jar_bridge::ID.into(),
        title: "Fabric Jar Bridge 0.1.0".into(),
        description: "Read Fabric JAR metadata; carry safe assets/data, not Java code".into(),
        keys: Vec::new(),
    });
}
