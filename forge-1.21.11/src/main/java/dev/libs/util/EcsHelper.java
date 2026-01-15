/*
 * LIBS - ECS Helper
 * Utility class for Entity Component System integration
 */
package dev.libs.util;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * ECS Helper - Manages entity data outside of mixin
 */
public class EcsHelper {

    private static final ConcurrentHashMap<Integer, EntityEcsData> ecsData = new ConcurrentHashMap<>();
    private static final AtomicInteger totalEntities = new AtomicInteger(0);
    private static final AtomicInteger parallelTicks = new AtomicInteger(0);

    /**
     * Register entity with ECS
     */
    public static void registerEntity(int entityId, double x, double y, double z) {
        EntityEcsData data = new EntityEcsData();
        data.x = x;
        data.y = y;
        data.z = z;
        data.velX = 0;
        data.velY = 0;
        data.velZ = 0;
        data.lastTickTime = System.nanoTime();

        ecsData.put(entityId, data);
        totalEntities.incrementAndGet();
    }

    /**
     * Update entity in ECS
     */
    public static void updateEntity(int entityId, double x, double y, double z,
            double velX, double velY, double velZ) {
        EntityEcsData data = ecsData.get(entityId);
        if (data != null) {
            data.x = x;
            data.y = y;
            data.z = z;
            data.velX = velX;
            data.velY = velY;
            data.velZ = velZ;
            data.lastTickTime = System.nanoTime();
        }
    }

    /**
     * Remove entity from ECS
     */
    public static void removeEntity(int entityId) {
        if (ecsData.remove(entityId) != null) {
            totalEntities.decrementAndGet();
        }
    }

    /**
     * Parallel tick all entities
     */
    public static void parallelTick(float deltaTime) {
        parallelTicks.incrementAndGet();

        ecsData.entrySet().parallelStream().forEach(entry -> {
            EntityEcsData data = entry.getValue();

            data.x += data.velX * deltaTime;
            data.y += data.velY * deltaTime;
            data.z += data.velZ * deltaTime;

            if (!data.onGround) {
                data.velY -= 0.08 * deltaTime;
            }
        });
    }

    /**
     * Get ECS data for entity
     */
    public static EntityEcsData getEcsData(int entityId) {
        return ecsData.get(entityId);
    }

    /**
     * Check if entities are independent
     */
    public static boolean areIndependent(int entityId1, int entityId2) {
        EntityEcsData data1 = ecsData.get(entityId1);
        EntityEcsData data2 = ecsData.get(entityId2);

        if (data1 == null || data2 == null)
            return true;

        int chunkX1 = (int) data1.x >> 4;
        int chunkZ1 = (int) data1.z >> 4;
        int chunkX2 = (int) data2.x >> 4;
        int chunkZ2 = (int) data2.z >> 4;

        return Math.abs(chunkX1 - chunkX2) > 1 || Math.abs(chunkZ1 - chunkZ2) > 1;
    }

    /**
     * Get ECS statistics
     */
    public static String getStats() {
        return String.format("ECS: %d entities, %d parallel ticks",
                totalEntities.get(), parallelTicks.get());
    }

    /**
     * ECS data structure (DOD layout)
     */
    public static class EntityEcsData {
        public double x, y, z;
        public double velX, velY, velZ;
        public float health;
        public boolean onGround;
        public int entityType;
        public long lastTickTime;
    }
}
