/*
 * LIBS - Netcode Helper
 * Utility class for predictive netcode
 */
package dev.libs.util;

import net.minecraft.world.phys.Vec3;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Netcode Helper - Entity prediction and interpolation
 */
public class NetcodeHelper {

    private static final ConcurrentHashMap<Integer, EntityPrediction> predictions = new ConcurrentHashMap<>();
    private static long estimatedPing = 50;

    /**
     * Predict entity position based on velocity and ping
     */
    public static Vec3 predictPosition(Vec3 current, Vec3 velocity) {
        double predictAhead = estimatedPing / 1000.0;
        return current.add(
                velocity.x * predictAhead,
                velocity.y * predictAhead,
                velocity.z * predictAhead);
    }

    /**
     * Smoothly interpolate entity to target position
     */
    public static Vec3 interpolatePosition(int entityId, Vec3 serverPos, Vec3 currentPos) {
        EntityPrediction prediction = predictions.computeIfAbsent(entityId, k -> new EntityPrediction());

        prediction.targetPos = serverPos;
        prediction.lastUpdateTime = System.currentTimeMillis();

        if (prediction.lastPos == null) {
            prediction.lastPos = serverPos;
            return serverPos;
        }

        float speed = 0.3f;

        Vec3 interpolated = new Vec3(
                lerp(currentPos.x, serverPos.x, speed),
                lerp(currentPos.y, serverPos.y, speed),
                lerp(currentPos.z, serverPos.z, speed));

        double error = interpolated.distanceTo(serverPos);
        if (error > 5.0) {
            return serverPos;
        }

        prediction.lastPos = interpolated;
        return interpolated;
    }

    private static double lerp(double a, double b, float t) {
        return a + (b - a) * t;
    }

    /**
     * Update ping estimate
     */
    public static void updatePing(long pingMs) {
        estimatedPing = (long) (estimatedPing * 0.8 + pingMs * 0.2);
    }

    /**
     * Get current ping
     */
    public static long getPing() {
        return estimatedPing;
    }

    /**
     * Get netcode stats
     */
    public static String getStats() {
        return String.format("Netcode: ping=%dms entities=%d",
                estimatedPing, predictions.size());
    }

    /**
     * Clear prediction
     */
    public static void clearPrediction(int entityId) {
        predictions.remove(entityId);
    }

    private static class EntityPrediction {
        Vec3 lastPos;
        Vec3 targetPos;
        long lastUpdateTime;
    }
}
