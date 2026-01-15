/*
 * LIBS - Audio Helper
 * Utility class for ray-traced audio
 */
package dev.libs.util;

import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.phys.Vec3;

/**
 * Audio Helper - Ray-traced audio calculations
 */
public class AudioHelper {

    private static float lastReverbDecay = 1.0f;
    private static float lastReverbWet = 0.3f;

    /**
     * Calculate occlusion between listener and sound source
     */
    public static float calculateOcclusion(double sourceX, double sourceY, double sourceZ) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.level == null || mc.player == null)
            return 1.0f;

        Vec3 listener = mc.player.getEyePosition();
        Vec3 source = new Vec3(sourceX, sourceY, sourceZ);
        Vec3 direction = source.subtract(listener).normalize();
        double distance = listener.distanceTo(source);

        if (distance < 2.0)
            return 1.0f;
        if (distance > 32.0)
            return 0.1f;

        float occlusion = 0.0f;

        for (double d = 1.0; d < distance; d += 0.5) {
            Vec3 pos = listener.add(direction.scale(d));
            BlockPos blockPos = BlockPos.containing(pos);

            if (!mc.level.getBlockState(blockPos).isAir()) {
                float absorption = getMaterialAbsorption(mc.level.getBlockState(blockPos).getBlock().toString());
                occlusion += absorption;
            }

            if (occlusion >= 1.0f)
                break;
        }

        return 1.0f - Math.min(occlusion, 0.9f);
    }

    /**
     * Get absorption coefficient for material
     */
    private static float getMaterialAbsorption(String blockName) {
        String name = blockName.toLowerCase();

        if (name.contains("wool"))
            return 0.7f;
        if (name.contains("glass"))
            return 0.1f;
        if (name.contains("wood") || name.contains("plank"))
            return 0.2f;
        if (name.contains("stone") || name.contains("brick"))
            return 0.05f;
        if (name.contains("water"))
            return 0.3f;
        if (name.contains("leaves"))
            return 0.4f;

        return 0.15f;
    }

    /**
     * Calculate reverb based on room geometry
     */
    public static void calculateReverb() {
        Minecraft mc = Minecraft.getInstance();
        if (mc.level == null || mc.player == null)
            return;

        Vec3 pos = mc.player.getEyePosition();
        float totalDistance = 0;
        int hits = 0;
        float absorptionSum = 0;

        Vec3[] directions = {
                new Vec3(1, 0, 0), new Vec3(-1, 0, 0),
                new Vec3(0, 1, 0), new Vec3(0, -1, 0),
                new Vec3(0, 0, 1), new Vec3(0, 0, -1)
        };

        for (Vec3 dir : directions) {
            for (double d = 1; d < 32; d += 1) {
                Vec3 checkPos = pos.add(dir.scale(d));
                BlockPos blockPos = BlockPos.containing(checkPos);

                if (!mc.level.getBlockState(blockPos).isAir()) {
                    totalDistance += (float) d;
                    hits++;
                    absorptionSum += getMaterialAbsorption(
                            mc.level.getBlockState(blockPos).getBlock().toString());
                    break;
                }
            }
        }

        if (hits > 0) {
            float avgDistance = totalDistance / hits;
            float avgAbsorption = absorptionSum / hits;

            lastReverbDecay = avgDistance * 0.1f * (1.0f - avgAbsorption);
            lastReverbWet = (avgDistance / 32.0f) * 0.5f;
        }
    }

    /**
     * Get audio stats
     */
    public static String getStats() {
        return String.format("Reverb: decay=%.2fs wet=%.2f", lastReverbDecay, lastReverbWet);
    }
}
