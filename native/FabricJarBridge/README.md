# Fabric Jar Bridge — experimental Java execution 0.2.0

**Scope:** put a local `.jar` in the **built native instance's** `horizon-mods/` directory and **restart the native game**. This bridge detects Fabric metadata and carries compatible *assets and data* into Horizon's content overlay automatically at game startup. The generated cache is `.horizon-cache/fabric-jar-bridge/*.hmod` and uses immutable SHA-256 based names.

**Experimental Java sidecar, not a full Fabric Loader.** Version 0.2.0 can invoke actual Java `main` and `client` Fabric entrypoint bytecode after an explicit SHA-256 per-JAR opt-in. A deliberately minimal Java shim implements `net.fabricmc.api.ModInitializer`, `ClientModInitializer` and `dev.horizon.fabricbridge.HorizonBridge.registerCommand`. Most real mods will fail because Minecraft's Java classes, Fabric API modules, Mixins, class mappings, and mod dependencies are not implemented. Do not claim compatibility with Sodium, Jade, AppleSkin etc. The launcher mod browser is unchanged.

## Why this implementation

Horizon already has a separate upstream `horizon-port-pack` utility (`crates/horizon-port-pack`) and a `check <jar> <out>` workflow that distinguishes portable content from Java behavior. This independent `.hnmod` follows the same principle, but uses the ZIP, JSON and content-overlay APIs available in the exact pinned game so players can put a JAR in the game's directory and see suitable data without a separate manual porting step. The approved Java path launches the user's installed `javac` and `java` programs. It never downloads them; without a locally installed JDK, only data/assets import works.

## Compatibility

- Game: Minecraft **26.3**
- Horizon source: `8474ac55c9f6f581e2bb984af9efcb18666bdbb2` (**exact source revision**, required by startup hook)
- Horizon Mixins: `3f99a5c60dfa311cc365524c44ae2fd799bfa1de`
- Rust: **1.99.1** in the native client build
- Client-side instance; optional separate local JDK 11+ sidecar process, not a Java server

## Capabilities

1. Scans at most 64 direct `*.jar` files in the current native game instance's `horizon-mods/`.
2. Reads `fabric.mod.json` and validates the Fabric mod ID (Horizon-compatible lower-case letters, digits or underscores).
3. Carries bounded `assets/<id>/...` and `data/<id>/...`, plus allowed `data/minecraft/tags/...` and `data/c/tags/...`.
4. Uses Horizon's own `Content::read` validation and namespace conflict check and builds **one combined resource overlay**, retaining standard enabled `.hmod` content.
5. Reports ignored `.class` files for the resource importer; the separate, opt-in Java runner can invoke supported real Fabric entrypoint bytecode.
6. Detects changes by SHA-256 of the JAR and writes stable cache filenames; no generated content is committed to GitHub.
7. With explicit approval per exact SHA-256, starts a separate local Java process and invokes Fabric `main`/`client` entrypoints. A mod can register a real Horizon chat command using the tiny bridge API.

Safety caps: max JAR 128 MiB, 4096 entries, max copied file 8 MiB, max 64 MiB content, compression ratio cap, rejected symlinks, invalid archive paths, duplicate content paths and forbidden namespace overrides. Original JARs are never modified.

**Limitations:** special version-specific datapacks may not be understood by Minecraft 26.3; copying a model or recipe cannot register its code-defined block/item; most Java-only mods cannot run because they rely on missing Minecraft/Fabric Java classes. Unsupported Fabric IDs such as `fabric-api` are explicitly rejected. The launcher still doesn't list raw JAR files as enabled native mods. Disable/remove a JAR and restart to remove carried content. Keep backups of worlds when testing content mods.

## Installation and test on Windows

1. Build/install **FabricJarBridge.hnmod** with the native mods option in the Horizon launcher. The launcher requires a trust confirmation and rebuilds its own native instance.
2. Open **the instance that the native build actually launches**. Locate its `horizon-mods/` folder, not the original game's `horizon-mods/`.
3. Copy a Fabric JAR into that folder. Do not extract it. Untrusted JARs remain in resource-only mode.
4. Restart the **built native** game so resource preparation sees the new JAR.
5. In chat use `/fabricbridge list` (or `/fabricbridge help`). A JAR with a supported entrypoint shows `/fabricbridge trust <id>`. **Only approve JARs you trust with full access to your PC**, because they execute arbitrary Java code with your user rights. Approval is keyed to the exact SHA-256 of the JAR.
6. After approval, restart the game. If JDK 11+ (`javac` and `java`) is on PATH, the bridge invokes the approved Fabric entrypoint. Look for `Java reel execute` or a precise failure such as `NoClassDefFoundError`. A modified JAR needs fresh approval.
7. To remove it, remove the JAR from the native instance and restart. To revoke approval, delete that JAR's line from `horizon-mods/fabricbridge-trusted.txt`.

**Java proof test (CI):** create a real compiled Java JAR with `fabric.mod.json` pointing at `test.ActualFabricMod`, whose bytecode implements `net.fabricmc.api.ModInitializer` and calls `dev.horizon.fabricbridge.HorizonBridge.registerCommand("javaproof", "Java Fabric a execute son bytecode")`. The JDK compiler produces actual `.class` files; the real Java launcher invokes the initializer. In Horizon, an equivalent opt-in mod registers `/javaproof`, which responds in chat. The existing Rust resource import test is retained.

## Source and build

```sh
python3 -m unittest discover -s scripts/tests -p test_fabric_jar_bridge.py -v
python3 scripts/build_fabric_jar_bridge.py pack
python3 scripts/build_fabric_jar_bridge.py verify
(cd dist && sha256sum -c FabricJarBridge.hnmod.sha256)
```

GitHub Actions `.github/workflows/fabric-jar-bridge.yml` additionally invokes `horizon-mixins apply`, `cargo check`, and the embedded Rust tests against the pinned sources. Archive success alone does not establish Windows runtime compatibility or compatibility with arbitrary Fabric mods.

## License and attribution

No third-party JARs, Java classes or Mojang/Fabric textures are bundled or redistributed. The bridge imports resources from JARs the player already has, locally. Consult each JAR's license before redistributing converted resources.

The bridge is independent of HorizonUtilities and Status Effect Atelier. No launcher or official Horizon source files are committed in this repository.

## Supported Java API, limitations and safety

`dev.horizon.fabricbridge.HorizonBridge.registerCommand(String command, String reply)` is **an experimental API for bridge-aware Fabric mods**, not an official Fabric API method. A mod must have an ordinary public, no-argument `ModInitializer` or `ClientModInitializer` class. The runner invokes each entrypoint in a **short-lived separate JVM** and captures its command registrations; it does not keep a JVM alive for ticks/events or support Fabric Loader APIs, Mixins, Java Minecraft internals, rendering, game registry mutation, dependencies or Java's other loader adapters.

Run `/fabricbridge trust <id>` only for personally verified JARs; the user explicitly grants that exact JAR **arbitrary local code execution**, including filesystem and network access. Java is not sandboxed. The bridge enforces a startup timeout but that is not a security boundary. The exact SHA-256 approval prevents a modified JAR from silently inheriting approval. This experiment is independent of the ordinary assets/data carry path. For full compatibility, a stable process/event IPC layer and extensive Minecraft/Fabric API shims must still be implemented.
