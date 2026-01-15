/*
 * LIBS - Texture Manager Mixin
 * Minimal hooks - logic in MemoryHelper
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import net.minecraft.client.renderer.texture.TextureManager;

/**
 * TextureManager Mixin - Minimal hooks
 * Full logic in dev.libs.util.MemoryHelper
 */
@Mixin(TextureManager.class)
public abstract class TextureManagerMixin {
    // Logic moved to MemoryHelper utility class
}
