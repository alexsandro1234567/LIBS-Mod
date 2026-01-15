/*
 * LIBS - Sound Engine Mixin
 * Minimal hooks - logic in AudioHelper
 */
package dev.libs.mixin;

import org.spongepowered.asm.mixin.Mixin;
import net.minecraft.client.sounds.SoundEngine;

/**
 * SoundEngine Mixin - Minimal hooks
 * Full logic in dev.libs.util.AudioHelper
 */
@Mixin(SoundEngine.class)
public abstract class SoundEngineMixin {
    // Logic moved to AudioHelper utility class
}
