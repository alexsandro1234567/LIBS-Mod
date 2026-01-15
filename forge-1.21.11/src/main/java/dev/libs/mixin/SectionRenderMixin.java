/*
 * LIBS - Section Render Mixin
 * Minimal hooks - logic in LodHelper
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import net.minecraft.client.renderer.chunk.SectionRenderDispatcher;

/**
 * SectionRenderDispatcher Mixin - Minimal hooks
 * Full logic in dev.libs.util.LodHelper
 */
@Mixin(SectionRenderDispatcher.class)
public abstract class SectionRenderMixin {
    // Logic moved to LodHelper utility class
}
