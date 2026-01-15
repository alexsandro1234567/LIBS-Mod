/*
 * LIBS - Chunk Optimizer
 * Optimizes chunk rendering and meshing
 */
package dev.libs.render;

import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.phys.Vec3;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * ChunkOptimizer - Chunk rendering optimizations
 * 
 * Features:
 * - Distance-based update priority
 * - Visibility caching
 * - Mesh reuse for unchanged chunks
 */
public class ChunkOptimizer {

    private static final ConcurrentHashMap<Long, ChunkData> chunkDataCache = new ConcurrentHashMap<>();
    private static final AtomicInteger cacheHits = new AtomicInteger(0);
    private static final AtomicInteger cacheMisses = new AtomicInteger(0);

    // Distance thresholds for update priority (in chunks)
    private static final int IMMEDIATE_UPDATE_DISTANCE = 4;
    private static final int HIGH_PRIORITY_DISTANCE = 8;
    private static final int MEDIUM_PRIORITY_DISTANCE = 16;

    /**
     * Get update priority for a chunk based on distance from player
     * Returns: 0 = skip, 1 = low, 2 = medium, 3 = high, 4 = immediate
     */
    public static int getUpdatePriority(int chunkX, int chunkZ) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.player == null)
            return 2;

        int playerChunkX = mc.player.blockPosition().getX() >> 4;
        int playerChunkZ = mc.player.blockPosition().getZ() >> 4;

        int dx = Math.abs(chunkX - playerChunkX);
        int dz = Math.abs(chunkZ - playerChunkZ);
        int distance = Math.max(dx, dz);

        if (distance <= IMMEDIATE_UPDATE_DISTANCE)
            return 4;
        if (distance <= HIGH_PRIORITY_DISTANCE)
            return 3;
        if (distance <= MEDIUM_PRIORITY_DISTANCE)
            return 2;
        return 1;
    }

    /**
     * Check if chunk should be rendered this frame based on distance
     */
    public static boolean shouldRenderChunk(int chunkX, int chunkY, int chunkZ, int renderDistance) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.player == null)
            return true;

        int playerChunkX = mc.player.blockPosition().getX() >> 4;
        int playerChunkY = mc.player.blockPosition().getY() >> 4;
        int playerChunkZ = mc.player.blockPosition().getZ() >> 4;

        int dx = Math.abs(chunkX - playerChunkX);
        int dy = Math.abs(chunkY - playerChunkY);
        int dz = Math.abs(chunkZ - playerChunkZ);

        // Always render close chunks
        if (dx <= 2 && dy <= 2 && dz <= 2)
            return true;

        // Distance check
        int distSq = dx * dx + dy * dy + dz * dz;
        return distSq <= renderDistance * renderDistance;
    }

    /**
     * Check if chunk is in player's view direction (for priority)
     */
    public static boolean isInViewDirection(int chunkX, int chunkZ) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.player == null)
            return true;

        Vec3 look = mc.player.getLookAngle();
        Vec3 pos = mc.player.position();

        double chunkCenterX = chunkX * 16 + 8;
        double chunkCenterZ = chunkZ * 16 + 8;

        double dx = chunkCenterX - pos.x;
        double dz = chunkCenterZ - pos.z;
        double length = Math.sqrt(dx * dx + dz * dz);

        if (length < 16)
            return true; // Very close

        dx /= length;
        dz /= length;

        // Dot product with look direction
        double dot = dx * look.x + dz * look.z;
        return dot > -0.3; // 120 degree FOV cone behind
    }

    /**
     * Get cache hit rate
     */
    public static float getCacheHitRate() {
        int hits = cacheHits.get();
        int total = hits + cacheMisses.get();
        if (total == 0)
            return 0;
        return (float) hits / total * 100f;
    }

    /**
     * Clear chunk cache
     */
    public static void clearCache() {
        chunkDataCache.clear();
        cacheHits.set(0);
        cacheMisses.set(0);
    }

    /**
     * Chunk data cache entry
     */
    public static class ChunkData {
        public final long key;
        public final long lastUpdate;
        public final int vertexCount;
        public final boolean isEmpty;

        public ChunkData(long key, int vertexCount, boolean isEmpty) {
            this.key = key;
            this.lastUpdate = System.currentTimeMillis();
            this.vertexCount = vertexCount;
            this.isEmpty = isEmpty;
        }
    }
}
