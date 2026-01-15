/*
 * LIBS - Render Stats Helper
 * Utility class for chunk rendering statistics
 */
package dev.libs.util;

/**
 * Render Stats Helper - Chunk rendering statistics
 */
public class RenderStatsHelper {

    private static int chunksRendered = 0;
    private static int chunksSkipped = 0;
    private static long lastStatsReset = 0;

    /**
     * Reset stats every second
     */
    public static void resetIfNeeded() {
        long now = System.currentTimeMillis();
        if (now - lastStatsReset > 1000) {
            chunksRendered = 0;
            chunksSkipped = 0;
            lastStatsReset = now;
        }
    }

    /**
     * Get chunk stats
     */
    public static String getChunkStats() {
        int total = chunksRendered + chunksSkipped;
        if (total == 0)
            return "Chunks: 0/0 skipped";
        int percent = (chunksSkipped * 100) / total;
        return String.format("Chunks: %d/%d skipped (%d%%)", chunksSkipped, total, percent);
    }

    /**
     * Increment rendered
     */
    public static void incrementRendered() {
        chunksRendered++;
    }

    /**
     * Increment skipped
     */
    public static void incrementSkipped() {
        chunksSkipped++;
    }
}
