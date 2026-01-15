/*
 * LIBS - Entity Mixin with REAL culling
 * Prevents rendering of entities that shouldn't be visible
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

import net.minecraft.world.entity.Entity;
import net.minecraft.client.Minecraft;
import dev.libs.util.CullingHelper;

/**
 * Entity Mixin - Aggressive entity culling
 */
@Mixin(Entity.class)
public abstract class EntityMixin {

    /**
     * Hook into shouldRender to cull entities aggressively
     */
    @Inject(method = "shouldRender", at = @At("HEAD"), cancellable = true)
    private void libs_shouldRender(double camX, double camY, double camZ, CallbackInfoReturnable<Boolean> cir) {
        Entity self = (Entity) (Object) this;

        // Use our aggressive culling
        if (!CullingHelper.shouldRenderEntity(self, camX, camY, camZ)) {
            cir.setReturnValue(false);
        }
    }
}
