# Fabric Bridge compatibility ledger — experimental 0.4.0

This file describes code actually implemented in this branch, not a universal Fabric promise.

| Java symbol / feature | Status | Backing implementation |
| --- | --- | --- |
| `net.fabricmc.api.ModInitializer` | Narrow support | Invoked in the approved JVM sidecar |
| `net.fabricmc.api.ClientModInitializer` | Narrow support | Invoked in the approved JVM sidecar |
| `net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents.END_CLIENT_TICK` | Implemented subset | One Java callback per real Horizon native client tick |
| `net.minecraft.client.MinecraftClient.getInstance().player` | Implemented subset | Non-null in-world; coordinates from `Game::native_player` |
| `net.minecraft.client.network.ClientPlayerEntity.getX/Y/Z` | Implemented subset | Immutable snapshot updated from Horizon each client tick |
| `ClientPlayerEntity.getBlockPos/getBlockX/Y/Z` | Implemented subset | Floor of real Horizon position |
| `net.minecraft.util.math.BlockPos.getX/Y/Z` | Implemented subset | Snapshot value object |
| `dev.horizon.fabricbridge.HorizonBridge.registerCommand` | Bridge extension | Java stdout IPC to Horizon native command handler |
| Fabric Loader's full API and registry lifecycle | Unsupported | Partial local graph resolver only; no Fabric Loader runtime/registries |
| Fabric API other modules (events, item groups, rendering, networking) | Unsupported | No shims or matching Horizon handlers |
| Vanilla Java Minecraft classes beyond the listed facade | Unsupported | Horizon is implemented in Rust, not Java |
| Yarn/intermediary/official class names for arbitrary mods | Unsupported | No bytecode remapper |
| SpongePowered Fabric Mixins and access wideners | Unsupported | No Java class transformation or equivalent Rust native hook translation |
| Arbitrary Java Fabric mod JARs | Unsupported | Most mods depend on the above features |
| Native libraries in JARs | Unsupported | Not loaded by the bridge |
| Per-JAR SHA-256 approval | Required | `/fabricbridge trust <id>` plus restart |

## Architecture in this slice

- Horizon native `native_tick` hook reads the real player position (client side only).
- Bounded nonblocking Rust channel forwards events to one child JVM with a verified multi-JAR classpath ordered by supported dependency constraints.
- Java `Runner` initializes Fabric main/client entrypoints in dependency order, then consumes player-state tick messages.
- The minimal Java `MinecraftClient` facade is updated every tick and emits `END_CLIENT_TICK` callbacks.
- Java callbacks can register a native command through the limited `HorizonBridge` protocol.
- Each JAR runs with **full user OS privileges**, without a Java sandbox. Approval never means the JAR is safe.
- The Java subprocess remains alive between ticks, but its world data is limited to the exposed snapshot.

## Milestones required before claiming broader Fabric support

1. **Loader parity and metadata:** more `fabric.mod.json` entrypoint styles, dependency resolution, adapters, Fabric Loader API/mod containers, error handling.
2. **Minecraft façade:** systematically implement and test each requested Minecraft Java class and its semantics against Horizon native hooks; ensure world state, entities and threading match.
3. **Fabric API modules:** implement a vetted subset at a time, with cross-platform integration tests against real open-source Fabric mods.
4. **Mappings and Mixins:** handle deobfuscation/mappings, SpongePowered Mixin Java bytecode transformation, access wideners and target compatibility. A Java Mixin that targets an absent Java Minecraft class **cannot** be applied directly to Horizon Rust. Such changes require supported equivalents on Horizon's Rust hooks.
5. **Game state and lifecycle:** durable JVM lifecycle, events, command feedback, client/server separation, crash containment, disable/reload semantics.
6. **Compatibility claims:** maintain an explicit versioned per-mod matrix with real in-game Windows tests; unsupported mods must visibly report missing classes/API rather than claiming success.

**Not solved by shipping Minecraft's official Java game classes:** the original game is a different game runtime with its own state and rendering. Merely loading its bytecode alongside Horizon would not make Java mods control Horizon's Rust world. Respect Minecraft and Fabric distribution licenses; do not redistribute Mojang's game binaries.

## Version 0.4.0 : subset implemented

- Supported: graph of local approved JARs, common JVM classpath, deterministic topological order, library-only JAR, duplicate IDs/classes guard, `depends` and `breaks`/`conflicts`, version predicates numeric `*`, exact, comparisons, caret/tilde and conjunctions.
- Not supported: Fabric Loader language adapters, full SemVer prereleases, Fabric wildcard range syntax, nested JAR resolution, Fabric Loader API and Minecraft Java. Unsupported constraints fail closed. Approval SHA-256 per dependency remains mandatory.

- `net.fabricmc.loader.api.FabricLoader` (méthodes `getInstance`, `isModLoaded`, chemins jeu/config, environnement CLIENT, version brute) : **sous-ensemble ajouté en 0.4.0**, construit depuis la liste de JARs SHA-256 validés. Le reste de l'interface Fabric Loader est **non implémenté**.
