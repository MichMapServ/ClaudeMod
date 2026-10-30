# Fabric Jar Bridge — native Horizon 0.1.0

**Scope:** put a local `.jar` in the **built native instance's** `horizon-mods/` directory and **restart the native game**. This bridge detects Fabric metadata and carries compatible *assets and data* into Horizon's content overlay automatically at game startup. The generated cache is `.horizon-cache/fabric-jar-bridge/*.hmod` and uses immutable SHA-256 based names.

**Not a Fabric runtime.** It does not execute `.class` files, Fabric API, Mixin Java bytecode, loader entry points, native libraries or JVM mods. For a UI/gameplay mod such as Sodium, JEI, JourneyMap or Status Effect Bars, adding its JAR does **not** make its Java functionality run. That requires a dedicated Rust port or a separately engineered Java runtime, class/mapping bridge and event/API implementation. This bridge carries data only. It also does **not modify the Horizon launcher mod browser**: detection happens in the *game's* resource loading stage.

## Why this implementation

Horizon already has a separate upstream `horizon-port-pack` utility (`crates/horizon-port-pack`) and a `check <jar> <out>` workflow that distinguishes portable content from Java behavior. This independent `.hnmod` follows the same principle, but uses the ZIP, JSON and content-overlay APIs available in the exact pinned game so players can put a JAR in the game's directory and see suitable data without a separate manual porting step. No external executable is downloaded or launched by this mod.

## Compatibility

- Game: Minecraft **26.3**
- Horizon source: `8474ac55c9f6f581e2bb984af9efcb18666bdbb2` (**exact source revision**, required by startup hook)
- Horizon Mixins: `3f99a5c60dfa311cc365524c44ae2fd799bfa1de`
- Rust: **1.99.1** in the native client build
- Client-side instance; no Java runtime/server integration

## Capabilities

1. Scans at most 64 direct `*.jar` files in the current native game instance's `horizon-mods/`.
2. Reads `fabric.mod.json` and validates the Fabric mod ID (Horizon-compatible lower-case letters, digits or underscores).
3. Carries bounded `assets/<id>/...` and `data/<id>/...`, plus allowed `data/minecraft/tags/...` and `data/c/tags/...`.
4. Uses Horizon's own `Content::read` validation and namespace conflict check and builds **one combined resource overlay**, retaining standard enabled `.hmod` content.
5. Avoids executing Java code; reports number of `.class` files ignored and out-of-scope resources.
6. Detects changes by SHA-256 of the JAR and writes stable cache filenames; no generated content is committed to GitHub.

Safety caps: max JAR 128 MiB, 4096 entries, max copied file 8 MiB, max 64 MiB content, compression ratio cap, rejected symlinks, invalid archive paths, duplicate content paths and forbidden namespace overrides. Original JARs are never modified.

**Limitations:** special version-specific datapacks may not be understood by Minecraft 26.3; copying a model or recipe cannot register its code-defined block/item; Java-only mods won't appear as working mods. Unsupported Fabric IDs such as `fabric-api` are explicitly rejected. The launcher still doesn't list raw JAR files as enabled native mods. Disable/remove a JAR and restart to remove carried content. Keep backups of worlds when testing content mods.

## Installation and test on Windows

1. Build/install **FabricJarBridge.hnmod** with the native mods option in the Horizon launcher. The launcher requires a trust confirmation and rebuilds its own native instance.
2. Open **the instance that the native build actually launches**. Locate its `horizon-mods/` folder, not the original game's `horizon-mods/`.
3. Copy a Fabric JAR containing compatible *data/assets* into that folder. Do not extract it.
4. Restart the **built native** game so resource preparation sees the new JAR.
5. In chat use `/fabricbridge list` (or `/fabricbridge help`). The report names imported resources and tells you that Java classes were not executed.
6. To remove it, remove the JAR from the native instance and restart.

**Success test:** create/use a Fabric-format JAR with `fabric.mod.json` and `assets/<id>/lang/en_us.json` containing a custom translation key. The Rust CI unit test builds this JAR, converts it, parses it as a Horizon content package, and asserts that no Java class was copied.

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
