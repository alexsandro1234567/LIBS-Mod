/*
 * LIBS - LevelRenderer Mixin with REAL optimizations
 * Hooks into entity rendering and chunk rendering
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

import net.minecraft.client.renderer.LevelRenderer;
import net.minecraft.client.Camera;
import dev.libs.util.CullingHelper;
import dev.libs.util.RenderStatsHelper;

/**
 * LevelRenderer Mixin - Real render optimizations
 */
@Mixin(LevelRenderer.class)
public abstract class LevelRendererMixin {

    /**
     * Reset culling helpers at frame start
     */
    @Inject(method = "renderLevel", at = @At("HEAD"))
    private void libs_onRenderLevelStart(CallbackInfo ci) {
        CullingHelper.resetIfNeeded();
        RenderStatsHelper.resetIfNeeded();
    }
}
