fn horizon_utilities_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::horizon_utilities_native as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(), title: format!("Horizon Utilities {}", m::VERSION),
        description: "Survival, durability, inventory, waypoints and telemetry".into(),
        keys: vec![[ "toggle".into(), "Afficher / masquer".into(), "key.keyboard.n".into() ]],
    });
}
