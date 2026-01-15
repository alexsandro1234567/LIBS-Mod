/*
 * LIBS - Frustum Mixin with AGGRESSIVE Culling
 * Dramatically improves FPS by early-rejecting objects
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

import net.minecraft.client.renderer.culling.Frustum;
import net.minecraft.world.phys.AABB;
import net.minecraft.client.Minecraft;

/**
 * Aggressive Frustum Culling - Dramatically improves FPS
 * 
 * Optimizations:
 * - Fast distance-based early rejection
 * - Render distance culling
 * - Small object culling at distance
 */
@Mixin(Frustum.class)
public abstract class FrustumMixin {

    @Shadow
    private double camX;
    @Shadow
    private double camY;
    @Shadow
    private double camZ;

    /**
     * AGGRESSIVE visibility test - cull as much as possible EARLY
     */
    @Inject(method = "isVisible(Lnet/minecraft/world/phys/AABB;)Z", at = @At("HEAD"), cancellable = true)
    private void libs_aggressiveCull(AABB box, CallbackInfoReturnable<Boolean> cir) {
        // Calculate distance to box center
        double boxCenterX = (box.minX + box.maxX) * 0.5;
        double boxCenterY = (box.minY + box.maxY) * 0.5;
        double boxCenterZ = (box.minZ + box.maxZ) * 0.5;

        double dx = boxCenterX - camX;
        double dy = boxCenterY - camY;
        double dz = boxCenterZ - camZ;
        double distSq = dx * dx + dy * dy + dz * dz;

        // Get render distance from settings
        Minecraft mc = Minecraft.getInstance();
        if (mc.options == null)
            return;

        int renderDistance = mc.options.renderDistance().get();
        double renderDistSq = (renderDistance * 16.0) * (renderDistance * 16.0);

        // AGGRESSIVE: Cull objects beyond 90% of render distance
        if (distSq > renderDistSq * 0.81) {
            cir.setReturnValue(false);
            return;
        }

        // AGGRESSIVE: Very close objects always visible (no need to test)
        if (distSq < 64) { // 8 blocks
            cir.setReturnValue(true);
            return;
        }

        // AGGRESSIVE: Small objects at distance - cull them
        double boxSize = Math.max(
                Math.max(box.maxX - box.minX, box.maxY - box.minY),
                box.maxZ - box.minZ);

        // If object is small and far, skip it
        double dist = Math.sqrt(distSq);
        double screenSize = boxSize / dist * 1000; // Approximate pixel size

        if (screenSize < 2.0) { // Less than 2 pixels on screen
            cir.setReturnValue(false);
            return;
        }

        // Let vanilla handle the rest
    }
}
