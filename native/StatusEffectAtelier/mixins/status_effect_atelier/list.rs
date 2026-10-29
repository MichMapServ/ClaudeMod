fn status_effect_atelier_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: crate::game::status_effect_atelier::ID.into(),
        title: "Status Effect Atelier 0.1.0".into(),
        description: "Textured effect cards, timers, bar progress and configurable layouts".into(),
        keys: Vec::new(),
    });
}
