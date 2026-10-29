# Status Effect Atelier (native Horizon)

Separate mod for Minecraft 26.3; original effects overlay inspired by the concept of Status Effect Bars. No code or artwork from the Fabric mod is used.

## Interface

- Textured effect icons from Horizon's vanilla Minecraft asset atlas, path mob_effect/<id> (honors loaded resource packs).
- Original pixel-edged dark cards, shadows, colored side accent and duration progress bars.
- Effect names translated by Horizon; amplifier levels I through X and remaining time formatted from client tick rate.
- Three modes: cards, compact and icon-only; left/right HUD anchors, narrow/medium/wide cards and a configurable max effect count.
- Active and ambient effects, effects with infinite duration, beneficial vs harmful coloring, expiration highlight.
- Separate on/off switches for HUD and inventory. On small screens the inventory panel uses icons only.

## Settings

Options > Mods > Status Effect Atelier exposes enabled, HUD, inventory, mode, side, width, max visible cards (4/6/8/12), ambient filtering and very-long-duration filtering.

## Build and installation

Game: Minecraft 26.3. Exact Horizon source revision 8474ac55c9f6f581e2bb984af9efcb18666bdbb2. Horizon Mixins 3f99a5c60dfa311cc365524c44ae2fd799bfa1de. Rust 1.99.1.

GitHub Actions workflow status-effect-atelier packages StatusEffectAtelier.hnmod, verifies SHA-256, injects the mixins into the pinned Horizon client, runs cargo check and the embedded Rust tests.

To build the archive from this repository:

    python3 -m unittest discover -s scripts/tests -p test_status_atelier.py -v
    python3 scripts/build_status_atelier.py pack
    python3 scripts/build_status_atelier.py verify

To install, download the CI artifact, verify its SHA-256, put the .hnmod in the selected game's horizon-native-mods folder, enable Native mods (experimental) and build a native instance from Horizon Launcher. Alternatively extract the archive to a folder and use the official horizon-launcher native-build [FOLDER] --trust-native-code command. The launcher requires a user trust confirmation and recompiles Horizon; it cannot load .hnmod as a normal WebAssembly mod.

## Compatibility and limitations

- Two cancellable HEAD injections intercept Horizon's effect icon and inventory effects painters. This requires the EXACT source revision: it is not a stable-hooks-only mod and can interact with other mods targeting the same drawing functions.
- If disabled, the original Horizon renderer runs unchanged, including its inventory tooltips.
- Bar fill is relative to the longest *observed* duration of an effect, not an internal server-provided maximum. A refresh or hidden effect replacement may change this denominator.
- Client tick durations are estimates; tests of real-time rendering and interaction on Windows are still necessary after CI passes.
- No external PNG is shipped inside the .hnmod: native packaging allows only .rs/.toml. In-game effect icons are already textures and are loaded from the player's vanilla/resource-pack atlas; the new chrome is rendered pixel-by-pixel in Rust.

## Authorship

This is an original implementation. Status Effect Bars is a separate Fabric/Quilt project under LGPL-3.0-only. Its implementation and assets are not distributed by this mod.
