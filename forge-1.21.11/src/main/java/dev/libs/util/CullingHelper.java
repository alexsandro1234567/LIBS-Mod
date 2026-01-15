/*
 * LIBS - Aggressive Culling Helper
 * Uses native Rust DLL for fast culling calculations
 */
package dev.libs.util;

import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.culling.Frustum;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.core.BlockPos;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Aggressive Culling Helper - Entity and Particle culling
 * 
 * Features:
 * - Distance-based entity culling
 * - Occlusion culling (entities behind solid blocks)
 * - Screen-size culling (tiny entities at distance)
 * - Caching for frame coherence
 */
public class CullingHelper {

    // Stats
    private static final AtomicInteger totalEntities = new AtomicInteger(0);
    private static final AtomicInteger culledEntities = new AtomicInteger(0);
    private static volatile long lastReset = 0;

    // Visibility cache - cleared each frame
    private static final ConcurrentHashMap<Integer, Boolean> visibilityCache = new ConcurrentHashMap<>();
    private static volatile long lastFrameId = 0;

    // Configuration
    private static final double ENTITY_CULL_DISTANCE_SQ = 192 * 192; // 192 blocks
    private static final double SMALL_ENTITY_CULL_DISTANCE_SQ = 64 * 64; // 64 blocks for small entities
    private static final double ALWAYS_VISIBLE_DISTANCE_SQ = 16; // 4 blocks always visible

    /**
     * Reset stats periodically
     */
    public static void resetIfNeeded() {
        long now = System.currentTimeMillis();
        if (now - lastReset > 1000) {
            totalEntities.set(0);
            culledEntities.set(0);
            lastReset = now;
        }

        // Clear cache each frame
        long currentFrame = System.nanoTime() / 16_666_666L;
        if (currentFrame != lastFrameId) {
            visibilityCache.clear();
            lastFrameId = currentFrame;
        }
    }

    /**
     * AGGRESSIVE entity culling check
     * Returns true if entity should be rendered
     */
    public static boolean shouldRenderEntity(Entity entity, double camX, double camY, double camZ) {
        totalEntities.incrementAndGet();

        Minecraft mc = Minecraft.getInstance();
        if (mc.player == null)
            return true;

        // Always render the player
        if (entity == mc.player) {
            return true;
        }

        // Check cache first
        Boolean cached = visibilityCache.get(entity.getId());
        if (cached != null) {
            if (!cached)
                culledEntities.incrementAndGet();
            return cached;
        }

        // Calculate distance squared
        double dx = entity.getX() - camX;
        double dy = entity.getY() - camY;
        double dz = entity.getZ() - camZ;
        double distSq = dx * dx + dy * dy + dz * dz;

        // ALWAYS visible if very close
        if (distSq < ALWAYS_VISIBLE_DISTANCE_SQ) {
            visibilityCache.put(entity.getId(), true);
            return true;
        }

        // Get entity size
        AABB box = entity.getBoundingBox();
        double entitySize = Math.max(
                Math.max(box.maxX - box.minX, box.maxY - box.minY),
                box.maxZ - box.minZ);

        // Small entities get culled at shorter distances
        double cullDistSq = entitySize < 0.5 ? SMALL_ENTITY_CULL_DISTANCE_SQ : ENTITY_CULL_DISTANCE_SQ;

        // Distance culling
        if (distSq > cullDistSq) {
            visibilityCache.put(entity.getId(), false);
            culledEntities.incrementAndGet();
            return false;
        }

        // Screen-size culling - skip if too small on screen
        double dist = Math.sqrt(distSq);
        double screenSize = entitySize / dist * 1000;
        if (screenSize < 3.0) { // Less than 3 pixels
            visibilityCache.put(entity.getId(), false);
            culledEntities.incrementAndGet();
            return false;
        }

        // Occlusion check for far entities (behind solid blocks)
        if (distSq > 1024 && mc.level != null) { // >32 blocks
            Vec3 camPos = new Vec3(camX, camY, camZ);
            Vec3 entityPos = entity.position().add(0, entity.getBbHeight() * 0.5, 0);
            Vec3 dir = entityPos.subtract(camPos).normalize();

            // Check first 4 blocks in ray direction
            for (int i = 2; i < 6; i++) {
                BlockPos checkPos = BlockPos.containing(camPos.add(dir.scale(i)));
                if (mc.level.getBlockState(checkPos).isViewBlocking(mc.level, checkPos)) {
                    visibilityCache.put(entity.getId(), false);
                    culledEntities.incrementAndGet();
                    return false;
                }
            }
        }

        visibilityCache.put(entity.getId(), true);
        return true;
    }

    /**
     * Check if a particle should be rendered
     */
    public static boolean shouldRenderParticle(double x, double y, double z, double camX, double camY, double camZ) {
        double dx = x - camX;
        double dy = y - camY;
        double dz = z - camZ;
        double distSq = dx * dx + dy * dy + dz * dz;

        // Cull particles beyond 48 blocks (2304 sq)
        if (distSq > 2304) {
            return false;
        }

        // Random throttle for far particles (32+ blocks)
        if (distSq > 1024) {
            return (System.nanoTime() & 0x3) == 0; // Only render 25%
        }

        return true;
    }

    /**
     * Get culling stats
     */
    public static String getStats() {
        int total = totalEntities.get();
        int culled = culledEntities.get();
        if (total == 0)
            return "Entities: 0/0 culled";
        int percent = (culled * 100) / total;
        return String.format("Entities: %d/%d culled (%d%%)", culled, total, percent);
    }

    public static int getTotalEntities() {
        return totalEntities.get();
    }

    public static int getCulledEntities() {
        return culledEntities.get();
    }
}
