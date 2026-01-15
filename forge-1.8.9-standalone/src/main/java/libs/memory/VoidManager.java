/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * VoidManager.java - Off-Heap Memory Management System (GC Killer)
 * 
 * Manages memory outside the Java heap to avoid GC pauses.
 * Uses direct memory allocation for textures, models, and buffers.
 */

package dev.libs.memory;

import dev.libs.LibsConfig;
import dev.libs.bridge.ZeroCopyBuffer;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import sun.misc.Unsafe;

import java.lang.reflect.Field;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.Map;
import java.util.Queue;
import java.util.function.Consumer;

/**
 * VoidManager - Off-Heap Memory Management (The GC Killer)
 * 
 * This class manages memory outside the Java heap to:
 * - Avoid GC pauses that cause game stuttering
 * - Allow larger memory usage than Java heap limit
 * - Enable zero-copy transfers to native code
 * 
 * <h2>Memory Types:</h2>
 * <ul>
 * <li>Arena allocator - Fast bump allocation for temporary data</li>
 * <li>Pool allocator - Fixed-size block pools for common sizes</li>
 * <li>General allocator - Variable-size allocations</li>
 * </ul>
 * 
 * <h2>Thread Safety:</h2>
 * All allocation methods are thread-safe.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class VoidManager {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(VoidManager.class);

    /** Minimum allocation size */
    private static final int MIN_ALLOC_SIZE = 16;

    /** Maximum single allocation size (256 MB) */
    private static final long MAX_ALLOC_SIZE = 256L * 1024 * 1024;

    /** Arena size (16 MB per arena) */
    private static final long ARENA_SIZE = 16L * 1024 * 1024;

    /** Pool block sizes */
    private static final int[] POOL_SIZES = { 64, 256, 1024, 4096, 16384, 65536 };

    /** Memory warning threshold (90%) */
    private static final float WARNING_THRESHOLD = 0.9f;

    /** Memory critical threshold (95%) */
    private static final float CRITICAL_THRESHOLD = 0.95f;

    // =========================================================================
    // STATIC FIELDS
    // =========================================================================

    /** Unsafe instance for direct memory allocation */
    private static final Unsafe UNSAFE;

    /** Whether Unsafe is available */
    private static final boolean UNSAFE_AVAILABLE;

    static {
        Unsafe unsafe = null;
        try {
            Field field = Unsafe.class.getDeclaredField("theUnsafe");
            field.setAccessible(true);
            unsafe = (Unsafe) field.get(null);
        } catch (Exception e) {
            LOGGER.warn("Unsafe not available - using ByteBuffer fallback");
        }
        UNSAFE = unsafe;
        UNSAFE_AVAILABLE = unsafe != null;
    }

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Configuration */
    private final LibsConfig.MemoryConfig config;

    /** Whether manager is initialized */
    private final AtomicBoolean initialized = new AtomicBoolean(false);

    /** Whether manager is healthy */
    private final AtomicBoolean healthy = new AtomicBoolean(true);

    /** Maximum off-heap memory in bytes */
    private final long maxMemoryBytes;

    /** Current allocated bytes */
    private final AtomicLong allocatedBytes = new AtomicLong(0);

    /** Peak allocated bytes */
    private final AtomicLong peakBytes = new AtomicLong(0);

    /** Total allocation count */
    private final AtomicLong allocationCount = new AtomicLong(0);

    /** Total deallocation count */
    private final AtomicLong deallocationCount = new AtomicLong(0);

    /** Active allocations - maps pointer to size */
    private final ConcurrentHashMap<Long, AllocationInfo> activeAllocations = new ConcurrentHashMap<>();

    /** Arena allocators */
    private final ConcurrentHashMap<Long, Arena> arenas = new ConcurrentHashMap<>();

    /** Pool allocators by size */
    private final ConcurrentHashMap<Integer, Pool> pools = new ConcurrentHashMap<>();

    /** Pending deallocations (for deferred cleanup) */
    private final Queue<Long> pendingDeallocations = new ConcurrentLinkedQueue<>();

    /** Lock for critical operations */
    private final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

    /** Memory warning callback */
    private volatile Consumer<MemoryWarning> warningCallback;

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new VoidManager
     * 
     * @param config Memory configuration
     */
    public VoidManager(@NotNull LibsConfig.MemoryConfig config) {
        this.config = config;
        this.maxMemoryBytes = config.getOffHeapMaxMB() * 1024L * 1024L;

        LOGGER.debug("VoidManager created with max memory: {} MB", config.getOffHeapMaxMB());
    }

    // =========================================================================
    // LIFECYCLE
    // =========================================================================

    /**
     * Initialize the memory manager
     */
    public void initialize() {
        if (!initialized.compareAndSet(false, true)) {
            return;
        }

        LOGGER.info("Initializing VoidManager...");
        LOGGER.info("  Max off-heap memory: {} MB", maxMemoryBytes / (1024 * 1024));
        LOGGER.info("  Arena allocator: {}", config.isArenaAllocator() ? "enabled" : "disabled");
        LOGGER.info("  Pool allocator: {}", config.isPoolSmallObjects() ? "enabled" : "disabled");
        LOGGER.info("  Unsafe available: {}", UNSAFE_AVAILABLE);

        // Initialize pools if enabled
        if (config.isPoolSmallObjects()) {
            initializePools();
        }

        LOGGER.info("VoidManager initialized");
    }

    /**
     * Initialize memory pools
     */
    private void initializePools() {
        for (int size : POOL_SIZES) {
            // Calculate pool capacity based on size
            int capacity = Math.max(16, 1024 / (size / 64));
            pools.put(size, new Pool(size, capacity));
        }
        LOGGER.debug("Initialized {} memory pools", pools.size());
    }

    /**
     * Shutdown the memory manager
     */
    public void shutdown() {
        if (!initialized.compareAndSet(true, false)) {
            return;
        }

        LOGGER.info("Shutting down VoidManager...");

        // Process pending deallocations
        processPendingDeallocations();

        // Free all active allocations
        int freed = 0;
        for (Long ptr : activeAllocations.keySet()) {
            freeInternal(ptr);
            freed++;
        }
        activeAllocations.clear();

        // Free arenas
        for (Arena arena : arenas.values()) {
            arena.free();
        }
        arenas.clear();

        // Free pools
        for (Pool pool : pools.values()) {
            pool.free();
        }
        pools.clear();

        LOGGER.info("VoidManager shutdown complete. Freed {} allocations.", freed);
    }

    // =========================================================================
    // ALLOCATION - GENERAL
    // =========================================================================

    /**
     * Allocate memory
     * 
     * @param size Size in bytes
     * @return Pointer to allocated memory, or 0 if failed
     */
    public long allocate(long size) {
        return allocate(size, "general");
    }

    /**
     * Allocate memory with a tag for tracking
     * 
     * @param size Size in bytes
     * @param tag  Tag for debugging/tracking
     * @return Pointer to allocated memory, or 0 if failed
     */
    public long allocate(long size, String tag) {
        if (!initialized.get()) {
            LOGGER.warn("VoidManager not initialized");
            return 0;
        }

        if (size <= 0 || size > MAX_ALLOC_SIZE) {
            throw new IllegalArgumentException("Invalid allocation size: " + size);
        }

        // Round up to alignment
        long alignedSize = (size + 15) & ~15L;

        // Check memory limit
        if (!canAllocate(alignedSize)) {
            handleMemoryExhausted(alignedSize);
            return 0;
        }

        // Try pool allocation for small sizes
        if (config.isPoolSmallObjects() && alignedSize <= POOL_SIZES[POOL_SIZES.length - 1]) {
            long ptr = allocateFromPool((int) alignedSize);
            if (ptr != 0) {
                recordAllocation(ptr, alignedSize, tag, AllocationType.POOL);
                return ptr;
            }
        }

        // General allocation
        long ptr = allocateInternal(alignedSize);
        if (ptr != 0) {
            recordAllocation(ptr, alignedSize, tag, AllocationType.GENERAL);
        }

        return ptr;
    }

    /**
     * Allocate zeroed memory
     * 
     * @param size Size in bytes
     * @return Pointer to zeroed memory, or 0 if failed
     */
    public long allocateZeroed(long size) {
        long ptr = allocate(size);
        if (ptr != 0 && UNSAFE_AVAILABLE) {
            UNSAFE.setMemory(ptr, size, (byte) 0);
        }
        return ptr;
    }

    /**
     * Reallocate memory
     * 
     * @param ptr     Current pointer
     * @param newSize New size in bytes
     * @return New pointer (may be different), or 0 if failed
     */
    public long reallocate(long ptr, long newSize) {
        if (ptr == 0) {
            return allocate(newSize);
        }

        AllocationInfo info = activeAllocations.get(ptr);
        if (info == null) {
            LOGGER.warn("Reallocate called on unknown pointer: 0x{}", Long.toHexString(ptr));
            return allocate(newSize);
        }

        if (newSize <= info.size) {
            // Shrinking - just return same pointer
            return ptr;
        }

        // Allocate new, copy, free old
        long newPtr = allocate(newSize, info.tag);
        if (newPtr != 0 && UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(ptr, newPtr, info.size);
            free(ptr);
        }

        return newPtr;
    }

    // =========================================================================
    // ALLOCATION - INTERNAL
    // =========================================================================

    /**
     * Internal allocation using Unsafe or ByteBuffer
     */
    private long allocateInternal(long size) {
        if (UNSAFE_AVAILABLE) {
            try {
                return UNSAFE.allocateMemory(size);
            } catch (OutOfMemoryError e) {
                LOGGER.error("Failed to allocate {} bytes: {}", size, e.getMessage());
                return 0;
            }
        } else {
            // Fallback: use DirectByteBuffer (has overhead)
            try {
                ByteBuffer buffer = ByteBuffer.allocateDirect((int) size);
                // We can't get a real pointer without Unsafe, return buffer hashCode as
                // pseudo-pointer
                return buffer.hashCode() & 0xFFFFFFFFL;
            } catch (OutOfMemoryError e) {
                return 0;
            }
        }
    }

    /**
     * Allocate from pool
     */
    private long allocateFromPool(int size) {
        // Find smallest pool that fits
        for (int poolSize : POOL_SIZES) {
            if (poolSize >= size) {
                Pool pool = pools.get(poolSize);
                if (pool != null) {
                    return pool.allocate();
                }
            }
        }
        return 0;
    }

    /**
     * Record allocation in tracking structures
     */
    private void recordAllocation(long ptr, long size, String tag, AllocationType type) {
        AllocationInfo info = new AllocationInfo(size, tag, type, System.currentTimeMillis());
        activeAllocations.put(ptr, info);

        long total = allocatedBytes.addAndGet(size);
        allocationCount.incrementAndGet();

        // Update peak
        long peak = peakBytes.get();
        while (total > peak) {
            if (peakBytes.compareAndSet(peak, total)) {
                break;
            }
            peak = peakBytes.get();
        }

        // Check thresholds
        float usage = (float) total / maxMemoryBytes;
        if (usage >= CRITICAL_THRESHOLD) {
            handleMemoryCritical(total);
        } else if (usage >= WARNING_THRESHOLD) {
            handleMemoryWarning(total);
        }

        LOGGER.trace("Allocated {} bytes at 0x{} (tag={}, total={})",
                size, Long.toHexString(ptr), tag, total);
    }

    // =========================================================================
    // DEALLOCATION
    // =========================================================================

    /**
     * Free allocated memory
     * 
     * @param ptr Pointer to free
     */
    public void free(long ptr) {
        if (ptr == 0)
            return;

        AllocationInfo info = activeAllocations.remove(ptr);
        if (info == null) {
            LOGGER.warn("Free called on unknown pointer: 0x{}", Long.toHexString(ptr));
            return;
        }

        // Return to pool if applicable
        if (info.type == AllocationType.POOL) {
            Pool pool = findPoolForSize((int) info.size);
            if (pool != null && pool.free(ptr)) {
                allocatedBytes.addAndGet(-info.size);
                deallocationCount.incrementAndGet();
                return;
            }
        }

        // General free
        freeInternal(ptr);
        allocatedBytes.addAndGet(-info.size);
        deallocationCount.incrementAndGet();

        LOGGER.trace("Freed {} bytes at 0x{}", info.size, Long.toHexString(ptr));
    }

    /**
     * Defer freeing for later (for safe cross-thread cleanup)
     */
    public void deferFree(long ptr) {
        pendingDeallocations.add(ptr);
    }

    /**
     * Internal free
     */
    private void freeInternal(long ptr) {
        if (UNSAFE_AVAILABLE) {
            UNSAFE.freeMemory(ptr);
        }
        // ByteBuffer fallback doesn't need explicit free
    }

    /**
     * Find pool for a given size
     */
    private Pool findPoolForSize(int size) {
        for (int poolSize : POOL_SIZES) {
            if (poolSize >= size) {
                return pools.get(poolSize);
            }
        }
        return null;
    }

    // =========================================================================
    // GARBAGE COLLECTION
    // =========================================================================

    /**
     * Process pending deallocations and collect garbage
     */
    public void collectGarbage() {
        processPendingDeallocations();

        // Compact pools
        for (Pool pool : pools.values()) {
            pool.compact();
        }
    }

    /**
     * Process all pending deallocations
     */
    private void processPendingDeallocations() {
        Long ptr;
        int count = 0;
        while ((ptr = pendingDeallocations.poll()) != null) {
            free(ptr);
            count++;
        }
        if (count > 0) {
            LOGGER.debug("Processed {} deferred deallocations", count);
        }
    }

    // =========================================================================
    // MEMORY CHECKS
    // =========================================================================

    /**
     * Check if allocation is possible
     */
    private boolean canAllocate(long size) {
        return allocatedBytes.get() + size <= maxMemoryBytes;
    }

    /**
     * Handle memory exhaustion
     */
    private void handleMemoryExhausted(long requestedSize) {
        LOGGER.error("Memory exhausted! Requested: {} bytes, Used: {} / {} bytes",
                requestedSize, allocatedBytes.get(), maxMemoryBytes);

        // Try to free pending deallocations
        collectGarbage();

        // Fire warning
        if (warningCallback != null) {
            warningCallback.accept(new MemoryWarning(
                    MemoryWarning.Level.EXHAUSTED,
                    allocatedBytes.get(),
                    maxMemoryBytes,
                    "Memory exhausted, requested " + requestedSize + " bytes"));
        }
    }

    /**
     * Handle memory warning threshold
     */
    private void handleMemoryWarning(long usedBytes) {
        if (warningCallback != null) {
            warningCallback.accept(new MemoryWarning(
                    MemoryWarning.Level.WARNING,
                    usedBytes,
                    maxMemoryBytes,
                    "Memory usage above " + (int) (WARNING_THRESHOLD * 100) + "%"));
        }
    }

    /**
     * Handle critical memory threshold
     */
    private void handleMemoryCritical(long usedBytes) {
        healthy.set(false);

        if (warningCallback != null) {
            warningCallback.accept(new MemoryWarning(
                    MemoryWarning.Level.CRITICAL,
                    usedBytes,
                    maxMemoryBytes,
                    "CRITICAL: Memory usage above " + (int) (CRITICAL_THRESHOLD * 100) + "%"));
        }

        // Force garbage collection
        collectGarbage();
    }

    // =========================================================================
    // BUFFER CREATION
    // =========================================================================

    /**
     * Create a ZeroCopyBuffer backed by off-heap memory
     * 
     * @param size Size in bytes
     * @return ZeroCopyBuffer, or null if allocation failed
     */
    @Nullable
    public ZeroCopyBuffer createBuffer(long size) {
        long ptr = allocate(size);
        if (ptr == 0) {
            return null;
        }
        return ZeroCopyBuffer.wrapNative(ptr, size);
    }

    // =========================================================================
    // STATISTICS
    // =========================================================================

    /**
     * Get current allocated bytes
     */
    public long getAllocatedBytes() {
        return allocatedBytes.get();
    }

    /**
     * Get peak allocated bytes
     */
    public long getPeakBytes() {
        return peakBytes.get();
    }

    /**
     * Get maximum allowed bytes
     */
    public long getMaxBytes() {
        return maxMemoryBytes;
    }

    /**
     * Get usage percentage (0-100)
     */
    public float getUsagePercent() {
        return (float) allocatedBytes.get() / maxMemoryBytes * 100f;
    }

    /**
     * Get total allocation count
     */
    public long getAllocationCount() {
        return allocationCount.get();
    }

    /**
     * Get total deallocation count
     */
    public long getDeallocationCount() {
        return deallocationCount.get();
    }

    /**
     * Get active allocation count
     */
    public int getActiveAllocationCount() {
        return activeAllocations.size();
    }

    /**
     * Check if manager is healthy
     */
    public boolean isHealthy() {
        return healthy.get();
    }

    /**
     * Set warning callback
     */
    public void setWarningCallback(Consumer<MemoryWarning> callback) {
        this.warningCallback = callback;
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Allocation type enumeration
     */
    private enum AllocationType {
        GENERAL,
        POOL,
        ARENA
    }

    /**
     * Allocation tracking information
     */
    private static final class AllocationInfo {
        final long size;
        final String tag;
        final AllocationType type;
        final long timestamp;

        AllocationInfo(long size, String tag, AllocationType type, long timestamp) {
            this.size = size;
            this.tag = tag;
            this.type = type;
            this.timestamp = timestamp;
        }
    }

    /**
     * Memory pool for fixed-size allocations
     */
    private static final class Pool {
        private final int blockSize;
        private final Queue<Long> freeBlocks;
        private final AtomicLong totalBlocks;
        private volatile long poolMemory;

        Pool(int blockSize, int initialCapacity) {
            this.blockSize = blockSize;
            this.freeBlocks = new ConcurrentLinkedQueue<>();
            this.totalBlocks = new AtomicLong(0);

            // Pre-allocate blocks
            if (UNSAFE_AVAILABLE) {
                long size = (long) blockSize * initialCapacity;
                poolMemory = UNSAFE.allocateMemory(size);
                for (int i = 0; i < initialCapacity; i++) {
                    freeBlocks.add(poolMemory + (long) i * blockSize);
                }
                totalBlocks.set(initialCapacity);
            }
        }

        long allocate() {
            Long block = freeBlocks.poll();
            if (block != null) {
                return block;
            }

            // Pool exhausted - allocate new block directly
            if (UNSAFE_AVAILABLE) {
                long ptr = UNSAFE.allocateMemory(blockSize);
                totalBlocks.incrementAndGet();
                return ptr;
            }
            return 0;
        }

        boolean free(long ptr) {
            // Check if ptr is within pool memory
            if (poolMemory != 0 && ptr >= poolMemory &&
                    ptr < poolMemory + totalBlocks.get() * blockSize) {
                freeBlocks.add(ptr);
                return true;
            }
            // External allocation - free directly
            if (UNSAFE_AVAILABLE) {
                UNSAFE.freeMemory(ptr);
            }
            return true;
        }

        void compact() {
            // Nothing to do for simple pool
        }

        void free() {
            if (poolMemory != 0 && UNSAFE_AVAILABLE) {
                UNSAFE.freeMemory(poolMemory);
                poolMemory = 0;
            }
            freeBlocks.clear();
        }
    }

    /**
     * Arena allocator for fast bump allocation
     */
    private static final class Arena {
        private long memory;
        private final long size;
        private final AtomicLong offset;

        Arena(long size) {
            this.size = size;
            this.offset = new AtomicLong(0);

            if (UNSAFE_AVAILABLE) {
                this.memory = UNSAFE.allocateMemory(size);
            }
        }

        long allocate(long allocSize) {
            long newOffset = offset.addAndGet(allocSize);
            if (newOffset > size) {
                offset.addAndGet(-allocSize); // Roll back
                return 0;
            }
            return memory + newOffset - allocSize;
        }

        void reset() {
            offset.set(0);
        }

        void free() {
            if (memory != 0 && UNSAFE_AVAILABLE) {
                UNSAFE.freeMemory(memory);
                memory = 0;
            }
        }
    }

    /**
     * Memory warning information
     */
    public static final class MemoryWarning {
        public enum Level {
            WARNING, CRITICAL, EXHAUSTED
        }

        private final Level level;
        private final long usedBytes;
        private final long maxBytes;
        private final String message;

        MemoryWarning(Level level, long usedBytes, long maxBytes, String message) {
            this.level = level;
            this.usedBytes = usedBytes;
            this.maxBytes = maxBytes;
            this.message = message;
        }

        public Level getLevel() {
            return level;
        }

        public long getUsedBytes() {
            return usedBytes;
        }

        public long getMaxBytes() {
            return maxBytes;
        }

        public float getUsagePercent() {
            return (float) usedBytes / maxBytes * 100f;
        }

        public String getMessage() {
            return message;
        }
    }
}
