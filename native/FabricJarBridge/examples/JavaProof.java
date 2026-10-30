package proof;
import net.fabricmc.api.ModInitializer;
import dev.horizon.fabricbridge.HorizonBridge;

/**
 * Demonstration of actual Java bytecode executed by Fabric Jar Bridge 0.2.
 * Intentionally uses only the experimental Horizon bridge API.
 */
public final class JavaProof implements ModInitializer {
    @Override
    public void onInitialize() {
        HorizonBridge.registerCommand("javaproof", "Java Fabric execute dans Horizon !");
    }
}
