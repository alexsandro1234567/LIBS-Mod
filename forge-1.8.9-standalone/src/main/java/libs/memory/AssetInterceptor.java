/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * AssetInterceptor.java - Asset Deduplication and Caching
 * 
 * Intercepts Minecraft asset loading to deduplicate textures,
 * models, and sounds in off-heap memory.
 */

package dev.libs.memory;

import dev.libs.bridge.ZeroCopyBuffer;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

/**
 * AssetInterceptor - Asset Deduplication System
 * 
 * Intercepts Minecraft's asset loading to:
 * - Deduplicate identical assets (textures, sounds, models)
 * - Store assets in off-heap memory (VoidManager)
 * - Provide fast access via content-addressable hashing
 * - Track asset usage for smart eviction
 * 
 * <h2>How it works:</h2>
 * <ol>
 * <li>Asset data is hashed (SHA-256) when loaded</li>
 * <li>If hash exists, return existing pointer (deduplication)</li>
 * <li>If new, store in off-heap memory with reference counting</li>
 * <li>When no longer used, data is freed</li>
 * </ol>
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class AssetInterceptor {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(AssetInterceptor.class);

    /** Asset type enumeration */
    public enum AssetType {
        TEXTURE,
        MODEL,
        SOUND,
        SHADER,
        FONT,
        OTHER
    }

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Reference to VoidManager for memory allocation */
    private final VoidManager voidManager;

    /** Asset cache by hash */
    private final ConcurrentHashMap<String, AssetEntry> assetCache;

    /** Asset lookup by path */
    private final ConcurrentHashMap<String, String> pathToHash;

    /** Statistics */
    private final AtomicLong totalAssetsLoaded = new AtomicLong(0);
    private final AtomicLong totalDeduplicatedAssets = new AtomicLong(0);
    private final AtomicLong totalBytesStored = new AtomicLong(0);
    private final AtomicLong totalBytesSaved = new AtomicLong(0);

    /** MessageDigest for hashing (thread-local for thread safety) */
    private static final ThreadLocal<MessageDigest> digestLocal = ThreadLocal.withInitial(() -> {
        try {
            return MessageDigest.getInstance("SHA-256");
        } catch (Exception e) {
            throw new RuntimeException("SHA-256 not available", e);
        }
    });

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new AssetInterceptor
     * 
     * @param voidManager VoidManager for memory allocation
     */
    public AssetInterceptor(@NotNull VoidManager voidManager) {
        this.voidManager = Objects.requireNonNull(voidManager, "voidManager cannot be null");
        this.assetCache = new ConcurrentHashMap<>();
        this.pathToHash = new ConcurrentHashMap<>();

        LOGGER.debug("AssetInterceptor created");
    }

    // =========================================================================
    // LIFECYCLE
    // =========================================================================

    /**
     * Initialize the asset interceptor
     */
    public void initialize() {
        LOGGER.info("AssetInterceptor initialized");
    }

    /**
     * Shutdown the asset interceptor
     */
    public void shutdown() {
        clear();
        LOGGER.info("AssetInterceptor shutdown");
    }

    // =========================================================================
    // ASSET LOADING
    // =========================================================================

    /**
     * Store an asset and return a handle to it
     * 
     * @param path Resource path (e.g., "textures/blocks/stone.png")
     * @param data Asset data
     * @param type Asset type
     * @return Asset handle for accessing the stored data
     */
    @NotNull
    public AssetHandle store(@NotNull String path, @NotNull byte[] data, @NotNull AssetType type) {
        Objects.requireNonNull(path, "path cannot be null");
        Objects.requireNonNull(data, "data cannot be null");
        Objects.requireNonNull(type, "type cannot be null");

        totalAssetsLoaded.incrementAndGet();

        // Calculate hash
        String hash = calculateHash(data);

        // Check if already cached
        AssetEntry existing = assetCache.get(hash);
        if (existing != null) {
            // Deduplicated!
            existing.incrementRefCount();
            pathToHash.put(path, hash);

            totalDeduplicatedAssets.incrementAndGet();
            totalBytesSaved.addAndGet(data.length);

            LOGGER.debug("Asset deduplicated: {} ({} bytes saved)", path, data.length);
            return new AssetHandle(hash, existing.pointer, data.length, type);
        }

        // New asset - allocate memory
        long pointer = voidManager.allocate(data.length, "asset:" + type.name());
        if (pointer == 0) {
            LOGGER.error("Failed to allocate memory for asset: {} ({} bytes)", path, data.length);
            return AssetHandle.INVALID;
        }

        // Copy data to off-heap memory
        // Note: In production, this would use ZeroCopyBuffer or Unsafe
        // For now, we just track the pointer

        AssetEntry entry = new AssetEntry(pointer, data.length, type, hash);
        assetCache.put(hash, entry);
        pathToHash.put(path, hash);

        totalBytesStored.addAndGet(data.length);

        LOGGER.debug("Asset stored: {} ({} bytes) at 0x{}",
                path, data.length, Long.toHexString(pointer));

        return new AssetHandle(hash, pointer, data.length, type);
    }

    /**
     * Store an asset from a ByteBuffer
     */
    @NotNull
    public AssetHandle store(@NotNull String path, @NotNull ByteBuffer data, @NotNull AssetType type) {
        byte[] bytes = new byte[data.remaining()];
        data.get(bytes);
        return store(path, bytes, type);
    }

    /**
     * Get an asset by path
     * 
     * @param path Resource path
     * @return Asset handle, or null if not found
     */
    @Nullable
    public AssetHandle get(@NotNull String path) {
        String hash = pathToHash.get(path);
        if (hash == null) {
            return null;
        }

        AssetEntry entry = assetCache.get(hash);
        if (entry == null) {
            pathToHash.remove(path);
            return null;
        }

        return new AssetHandle(hash, entry.pointer, entry.size, entry.type);
    }

    /**
     * Get an asset by hash
     */
    @Nullable
    public AssetHandle getByHash(@NotNull String hash) {
        AssetEntry entry = assetCache.get(hash);
        if (entry == null) {
            return null;
        }
        return new AssetHandle(hash, entry.pointer, entry.size, entry.type);
    }

    /**
     * Check if an asset exists
     */
    public boolean exists(@NotNull String path) {
        return pathToHash.containsKey(path);
    }

    // =========================================================================
    // ASSET RELEASE
    // =========================================================================

    /**
     * Release a reference to an asset
     * 
     * @param path Resource path
     */
    public void release(@NotNull String path) {
        String hash = pathToHash.get(path);
        if (hash == null) {
            return;
        }

        AssetEntry entry = assetCache.get(hash);
        if (entry == null) {
            pathToHash.remove(path);
            return;
        }

        int newRefCount = entry.decrementRefCount();
        if (newRefCount <= 0) {
            // No more references - free memory
            voidManager.free(entry.pointer);
            assetCache.remove(hash);

            totalBytesStored.addAndGet(-entry.size);

            LOGGER.debug("Asset freed: {} ({} bytes)", path, entry.size);
        }

        pathToHash.remove(path);
    }

    /**
     * Release a reference by handle
     */
    public void release(@NotNull AssetHandle handle) {
        if (!handle.isValid()) {
            return;
        }

        AssetEntry entry = assetCache.get(handle.hash);
        if (entry == null) {
            return;
        }

        int newRefCount = entry.decrementRefCount();
        if (newRefCount <= 0) {
            voidManager.free(entry.pointer);
            assetCache.remove(handle.hash);
            totalBytesStored.addAndGet(-entry.size);
        }
    }

    // =========================================================================
    // BULK OPERATIONS
    // =========================================================================

    /**
     * Preload assets from a list of paths
     * 
     * @param paths        Paths to preload
     * @param dataProvider Function to load data by path
     * @param type         Asset type
     * @return Number of assets loaded
     */
    public int preload(@NotNull Iterable<String> paths,
            @NotNull java.util.function.Function<String, byte[]> dataProvider,
            @NotNull AssetType type) {
        int count = 0;
        for (String path : paths) {
            byte[] data = dataProvider.apply(path);
            if (data != null) {
                store(path, data, type);
                count++;
            }
        }
        LOGGER.info("Preloaded {} {} assets", count, type);
        return count;
    }

    /**
     * Clear all cached assets
     */
    public void clear() {
        for (AssetEntry entry : assetCache.values()) {
            voidManager.free(entry.pointer);
        }

        assetCache.clear();
        pathToHash.clear();

        totalBytesStored.set(0);

        LOGGER.info("Asset cache cleared");
    }

    // =========================================================================
    // HASHING
    // =========================================================================

    /**
     * Calculate SHA-256 hash of data
     */
    private String calculateHash(byte[] data) {
        MessageDigest digest = digestLocal.get();
        digest.reset();
        byte[] hashBytes = digest.digest(data);

        StringBuilder sb = new StringBuilder(64);
        for (byte b : hashBytes) {
            sb.append(String.format("%02x", b));
        }
        return sb.toString();
    }

    // =========================================================================
    // STATISTICS
    // =========================================================================

    /**
     * Get total assets loaded
     */
    public long getTotalAssetsLoaded() {
        return totalAssetsLoaded.get();
    }

    /**
     * Get number of deduplicated assets
     */
    public long getDeduplicatedAssets() {
        return totalDeduplicatedAssets.get();
    }

    /**
     * Get total bytes stored in cache
     */
    public long getTotalBytesStored() {
        return totalBytesStored.get();
    }

    /**
     * Get bytes saved by deduplication
     */
    public long getBytesSaved() {
        return totalBytesSaved.get();
    }

    /**
     * Get deduplication ratio (0-1)
     */
    public float getDeduplicationRatio() {
        long loaded = totalAssetsLoaded.get();
        if (loaded == 0)
            return 0;
        return (float) totalDeduplicatedAssets.get() / loaded;
    }

    /**
     * Get current cache size (unique assets)
     */
    public int getCacheSize() {
        return assetCache.size();
    }

    /**
     * Get statistics as a map
     */
    public Map<String, Object> getStatistics() {
        Map<String, Object> stats = new ConcurrentHashMap<>();
        stats.put("totalAssetsLoaded", totalAssetsLoaded.get());
        stats.put("uniqueAssets", assetCache.size());
        stats.put("deduplicatedAssets", totalDeduplicatedAssets.get());
        stats.put("deduplicationRatio", getDeduplicationRatio());
        stats.put("totalBytesStored", totalBytesStored.get());
        stats.put("bytesSaved", totalBytesSaved.get());
        stats.put("pathMappings", pathToHash.size());
        return stats;
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Internal asset entry
     */
    private static final class AssetEntry {
        final long pointer;
        final int size;
        final AssetType type;
        final String hash;
        final long createdAt;
        volatile long lastAccessedAt;
        private volatile int refCount;

        AssetEntry(long pointer, int size, AssetType type, String hash) {
            this.pointer = pointer;
            this.size = size;
            this.type = type;
            this.hash = hash;
            this.createdAt = System.currentTimeMillis();
            this.lastAccessedAt = createdAt;
            this.refCount = 1;
        }

        synchronized void incrementRefCount() {
            refCount++;
            lastAccessedAt = System.currentTimeMillis();
        }

        synchronized int decrementRefCount() {
            return --refCount;
        }

        synchronized int getRefCount() {
            return refCount;
        }
    }

    /**
     * Public handle to an asset
     */
    public static final class AssetHandle {
        public static final AssetHandle INVALID = new AssetHandle(null, 0, 0, null);

        private final String hash;
        private final long pointer;
        private final int size;
        private final AssetType type;

        AssetHandle(String hash, long pointer, int size, AssetType type) {
            this.hash = hash;
            this.pointer = pointer;
            this.size = size;
            this.type = type;
        }

        public boolean isValid() {
            return pointer != 0 && hash != null;
        }

        public String getHash() {
            return hash;
        }

        public long getPointer() {
            return pointer;
        }

        public int getSize() {
            return size;
        }

        public AssetType getType() {
            return type;
        }

        @Override
        public String toString() {
            if (!isValid()) {
                return "AssetHandle[INVALID]";
            }
            return String.format("AssetHandle[hash=%s..., ptr=0x%x, size=%d, type=%s]",
                    hash.substring(0, 8), pointer, size, type);
        }

        @Override
        public boolean equals(Object o) {
            if (this == o)
                return true;
            if (o == null || getClass() != o.getClass())
                return false;
            AssetHandle that = (AssetHandle) o;
            return Objects.equals(hash, that.hash);
        }

        @Override
        public int hashCode() {
            return hash != null ? hash.hashCode() : 0;
        }
    }
}
