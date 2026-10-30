package proof;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import dev.horizon.fabricbridge.HorizonBridge;

/**
 * Live client tick proof. Reads REAL player coordinates forwarded by Horizon.
 * No fake position values are baked into this mod.
 */
public final class ClientTickProof implements ClientModInitializer {
    private boolean announced;
    @Override
    public void onInitializeClient() {
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            if (!announced && client.player != null) {
                announced = true;
                var pos = client.player.getBlockPos();
                HorizonBridge.registerCommand("javapos",
                    "Player coordinates from Horizon: " + pos.getX() + " "
                    + pos.getY() + " " + pos.getZ());
            }
        });
    }
}
