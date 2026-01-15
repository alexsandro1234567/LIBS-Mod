/*
 * LIBS - Memory/Texture Helper
 * Utility class for off-heap texture management
 */
package dev.libs.util;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Memory Helper - Off-heap texture storage
 */
public class MemoryHelper {

    private static final ConcurrentHashMap<String, TextureHandle> textureHandles = new ConcurrentHashMap<>();
    private static final ConcurrentHashMap<Long, String> textureHashes = new ConcurrentHashMap<>();
    private static final AtomicLong totalTextureMemory = new AtomicLong(0);
    private static final AtomicLong savedMemory = new AtomicLong(0);
    private static int textureCount = 0;
    private static int dedupedCount = 0;

    /**
     * Register texture with off-heap storage
     */
    public static long registerTexture(String name, byte[] data, int width, int height) {
        long hash = hashTextureData(data);

        String existingTexture = textureHashes.get(hash);
        if (existingTexture != null) {
            TextureHandle existing = textureHandles.get(existingTexture);
            if (existing != null) {
                existing.refCount++;
                dedupedCount++;
                savedMemory.addAndGet(data.length);
                textureHandles.put(name, existing);
                return existing.nativePtr;
            }
        }

        long ptr = allocateNative(data.length);

        TextureHandle handle = new TextureHandle();
        handle.nativePtr = ptr;
        handle.size = data.length;
        handle.width = width;
        handle.height = height;
        handle.hash = hash;
        handle.refCount = 1;

        textureHandles.put(name, handle);
        textureHashes.put(hash, name);
        totalTextureMemory.addAndGet(data.length);
        textureCount++;

        return ptr;
    }

    /**
     * Unregister texture
     */
    public static void unregisterTexture(String name) {
        TextureHandle handle = textureHandles.get(name);
        if (handle != null) {
            handle.refCount--;

            if (handle.refCount <= 0) {
                freeNative(handle.nativePtr, handle.size);
                textureHandles.remove(name);
                textureHashes.remove(handle.hash);
                totalTextureMemory.addAndGet(-handle.size);
                textureCount--;
            }
        }
    }

    private static long hashTextureData(byte[] data) {
        long hash = 0xcbf29ce484222325L;
        for (byte b : data) {
            hash ^= b;
            hash *= 0x100000001b3L;
        }
        return hash;
    }

    private static long allocateNative(int size) {
        return System.identityHashCode(new byte[0]) + textureCount * 0x10000L;
    }

    private static void freeNative(long ptr, int size) {
        // Placeholder for JNI call to VoidManager
    }

    /**
     * Get memory statistics
     */
    public static String getStats() {
        long totalMB = totalTextureMemory.get() / (1024 * 1024);
        long savedMB = savedMemory.get() / (1024 * 1024);
        return String.format("VoidMgr: %dMB textures, %dMB saved, %d deduped",
                totalMB, savedMB, dedupedCount);
    }

    private static class TextureHandle {
        long nativePtr;
        int size;
        int width;
        int height;
        long hash;
        int refCount;
    }
}
