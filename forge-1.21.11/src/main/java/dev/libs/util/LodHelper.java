/*
 * LIBS - LOD Helper
 * Utility class for Nanite-style LOD
 */
package dev.libs.util;

import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;

/**
 * LOD Helper - Distance-based level of detail
 */
public class LodHelper {

    private static int chunksSkipped = 0;
    private static int chunksReduced = 0;
    private static long lastResetTime = 0;

    /**
     * Get LOD level based on distance from camera
     * 0 = Full, 1 = Medium, 2 = Low, 3 = Skip
     */
    public static int getLodLevel(BlockPos chunkOrigin) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.player == null)
            return 0;

        double dx = chunkOrigin.getX() + 8 - mc.player.getX();
        double dy = chunkOrigin.getY() + 8 - mc.player.getY();
        double dz = chunkOrigin.getZ() + 8 - mc.player.getZ();
        double distSq = dx * dx + dy * dy + dz * dz;

        if (distSq < 32 * 32)
            return 0;
        if (distSq < 128 * 128)
            return 1;
        if (distSq < 256 * 256)
            return 2;
        return 3;
    }

    /**
     * Check if chunk should be rebuilt
     */
    public static boolean shouldRebuild(BlockPos origin) {
        int lod = getLodLevel(origin);

        if (lod >= 3) {
            chunksSkipped++;
            return false;
        }

        return true;
    }

    /**
     * Get vertex reduction for LOD level
     */
    public static float getVertexReduction(int lod) {
        switch (lod) {
            case 0:
                return 1.0f;
            case 1:
                return 0.6f;
            case 2:
                return 0.3f;
            default:
                return 0.0f;
        }
    }

    /**
     * Get LOD statistics
     */
    public static String getStats() {
        long now = System.currentTimeMillis();
        if (now - lastResetTime > 1000) {
            chunksSkipped = 0;
            chunksReduced = 0;
            lastResetTime = now;
        }
        return String.format("LOD: %d skipped, %d reduced", chunksSkipped, chunksReduced);
    }
}
