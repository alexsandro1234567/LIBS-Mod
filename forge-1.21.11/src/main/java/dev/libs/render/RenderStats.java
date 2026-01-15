/*
 * LIBS - Rendering Statistics
 * Provides performance statistics for debugging
 */
package dev.libs.render;

import dev.libs.util.CullingHelper;
import dev.libs.util.RenderStatsHelper;

import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * RenderStats - Performance statistics tracking
 */
public class RenderStats {

    private static final AtomicLong frameCount = new AtomicLong(0);
    private static final AtomicLong lastFrameTime = new AtomicLong(System.nanoTime());
    private static final AtomicInteger fps = new AtomicInteger(0);
    private static final AtomicLong frameTimeAccum = new AtomicLong(0);
    private static final AtomicInteger frameCountForFps = new AtomicInteger(0);

    private static volatile float averageFrameTimeMs = 0;

    /**
     * Called at the start of each frame
     */
    public static void beginFrame() {
        frameCount.incrementAndGet();
    }

    /**
     * Called at the end of each frame
     */
    public static void endFrame() {
        long now = System.nanoTime();
        long elapsed = now - lastFrameTime.getAndSet(now);

        frameTimeAccum.addAndGet(elapsed);
        int count = frameCountForFps.incrementAndGet();

        // Update FPS every second
        if (frameTimeAccum.get() >= 1_000_000_000L) {
            fps.set(count);
            averageFrameTimeMs = (float) frameTimeAccum.get() / count / 1_000_000f;
            frameTimeAccum.set(0);
            frameCountForFps.set(0);
        }
    }

    /**
     * Get current FPS
     */
    public static int getFPS() {
        return fps.get();
    }

    /**
     * Get average frame time in milliseconds
     */
    public static float getAverageFrameTimeMs() {
        return averageFrameTimeMs;
    }

    /**
     * Get total frame count
     */
    public static long getFrameCount() {
        return frameCount.get();
    }

    /**
     * Get formatted statistics string
     */
    public static String getStatsString() {
        return String.format(
                "LIBS Stats | FPS: %d (%.2fms) | %s | %s",
                fps.get(),
                averageFrameTimeMs,
                CullingHelper.getStats(),
                RenderStatsHelper.getChunkStats());
    }

    /**
     * Reset all statistics
     */
    public static void reset() {
        frameCount.set(0);
        fps.set(0);
        frameTimeAccum.set(0);
        frameCountForFps.set(0);
        averageFrameTimeMs = 0;
    }
}
