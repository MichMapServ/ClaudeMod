# Real Java Fabric proof JAR

This demo is **not a normal Fabric mod compatible with unmodified Minecraft**:
it uses an experimental \`dev.horizon.fabricbridge.HorizonBridge\` API from the
Horizon Java sidecar. It proves that real Fabric \`ModInitializer\` Java bytecode
can execute and register a real Horizon command; it does not prove generic
Fabric mod compatibility.

1. Install **JDK 11+** with both \`javac\` and \`java\` on PATH.
2. From the repository root run \`python scripts/build_fabric_bridge_demo.py\`.
3. Place \`dist/FabricBridgeJavaProof.jar\` in the **native instance's**
   \`horizon-mods\` directory.
4. Run the native game and type \`/fabricbridge list\`; it says Java execution
   is disabled by default.
5. Type \`/fabricbridge trust fabricbridge_demo\`. This opts in the exact JAR
   SHA-256 (arbitrary Java code privileges!) and requires a restart.
6. Restart the native game. Type \`/javaproof\`.
   Expected reply: "Java Fabric execute dans Horizon !".
7. Enter a world and wait one tick, then type `/javapos`.
   It prints the player block coordinates captured by a Java `ClientTickEvents.END_CLIENT_TICK` callback from actual Horizon player state.

Do **not** use this to judge AppleSkin, Sodium or other Java mods: no Java
Minecraft class mappings, Fabric API modules or Mixins exist in this proof.
