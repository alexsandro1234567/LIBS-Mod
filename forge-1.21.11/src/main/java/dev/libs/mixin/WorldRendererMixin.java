/*
 * LIBS - WorldRenderer Mixin
 * Minimal hooks - logic in RenderStatsHelper
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

import net.minecraft.client.renderer.LevelRenderer;
import dev.libs.util.RenderStatsHelper;

/**
 * WorldRenderer Mixin - Minimal hooks
 * Full logic in dev.libs.util.RenderStatsHelper
 */
@Mixin(LevelRenderer.class)
public abstract class WorldRendererMixin {

    /**
     * Track camera position for distance calculations
     */
    @Inject(method = "prepareCullFrustum", at = @At("HEAD"))
    private void libs_onPrepareCullFrustum(CallbackInfo ci) {
        RenderStatsHelper.resetIfNeeded();
    }
}
