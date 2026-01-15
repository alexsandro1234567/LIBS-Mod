/*
 * LIBS - Client Packet Mixin
 * Minimal hooks - logic in NetcodeHelper
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import net.minecraft.client.multiplayer.ClientPacketListener;

/**
 * ClientPacketListener Mixin - Minimal hooks
 * Full logic in dev.libs.util.NetcodeHelper
 */
@Mixin(ClientPacketListener.class)
public abstract class ClientPacketMixin {
    // Logic moved to NetcodeHelper utility class
}
