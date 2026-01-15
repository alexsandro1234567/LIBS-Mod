/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * ChunkDataExtractor.java - Minecraft Chunk Data Extraction
 * 
 * Extracts block and biome data from Minecraft chunks for the native renderer.
 */

package dev.libs.world;

import dev.libs.bridge.NativeBridge;
import dev.libs.bridge.ZeroCopyBuffer;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.Queue;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.atomic.AtomicLong;

/**
 * ChunkDataExtractor - Chunk Data Pipeline
 * 
 * Extracts block data from Minecraft chunks and converts it to a format
 * suitable for the native Vulkan renderer. Handles:
 * 
 * <ul>
 * <li>Block IDs and metadata extraction</li>
 * <li>Biome data extraction</li>
 * <li>Light level extraction</li>
 * <li>Chunk meshing coordination</li>
 * </ul>
 * 
 * <h2>Data Format:</h2>
 * Each chunk section (16x16x16) is packed into a native buffer:
 * - Block IDs: 16-bit per block (4096 blocks = 8KB)
 * - Block states: 16-bit per block (8KB)
 * - Light levels: 4-bit + 4-bit per block (2KB)
 * - Total: ~18KB per section
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class ChunkDataExtractor {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(ChunkDataExtractor.class);

    /** Chunk dimensions */
    public static final int CHUNK_SIZE = 16;
    public static final int SECTION_HEIGHT = 16;
    public static final int BLOCKS_PER_SECTION = CHUNK_SIZE * CHUNK_SIZE * SECTION_HEIGHT;

    /** Maximum chunk queue size */
    private static final int MAX_QUEUE_SIZE = 256;

    /** Extraction priority */
    public enum Priority {
        IMMEDIATE, // Player's chunk
        HIGH, // Adjacent to player
        NORMAL, // Visible chunks
        LOW // Background loading
    }

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Native bridge reference */
    private final NativeBridge nativeBridge;

    /** Chunks pending extraction */
    private final ConcurrentLinkedQueue<ChunkRequest> extractionQueue;

    /** Currently loaded chunks (x,z packed as long) */
    private final Set<Long> loadedChunks;

    /** Chunks being processed */
    private final Set<Long> processingChunks;

    /** Statistics */
    private final AtomicLong chunksExtracted = new AtomicLong(0);
    private final AtomicLong sectionsProcessed = new AtomicLong(0);
    private final AtomicLong totalBytesProcessed = new AtomicLong(0);

    /** Buffer pool for extraction */
    private final Queue<ZeroCopyBuffer> bufferPool;

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new ChunkDataExtractor
     * 
     * @param nativeBridge Native bridge for sending data to engine
     */
    public ChunkDataExtractor(@NotNull NativeBridge nativeBridge) {
        this.nativeBridge = nativeBridge;
        this.extractionQueue = new ConcurrentLinkedQueue<>();
        this.loadedChunks = ConcurrentHashMap.newKeySet();
        this.processingChunks = ConcurrentHashMap.newKeySet();
        this.bufferPool = new ConcurrentLinkedQueue<>();

        // Pre-allocate some buffers
        for (int i = 0; i < 8; i++) {
            bufferPool.offer(ZeroCopyBuffer.allocate(32768)); // 32KB per buffer
        }

        LOGGER.debug("ChunkDataExtractor created");
    }

    // =========================================================================
    // LIFECYCLE
    // =========================================================================

    /**
     * Initialize the chunk data extractor
     */
    public void initialize() {
        LOGGER.info("ChunkDataExtractor initialized");
    }

    // =========================================================================
    // CHUNK REQUESTS
    // =========================================================================

    /**
     * Request chunk extraction
     * 
     * @param chunkX   Chunk X coordinate
     * @param chunkZ   Chunk Z coordinate
     * @param priority Extraction priority
     */
    public void requestChunk(int chunkX, int chunkZ, @NotNull Priority priority) {
        long key = packCoords(chunkX, chunkZ);

        // Skip if already loaded or processing
        if (loadedChunks.contains(key) || processingChunks.contains(key)) {
            return;
        }

        // Limit queue size
        if (extractionQueue.size() >= MAX_QUEUE_SIZE) {
            if (priority != Priority.IMMEDIATE) {
                return;
            }
            // For immediate, remove lowest priority item
            extractionQueue.poll();
        }

        extractionQueue.offer(new ChunkRequest(chunkX, chunkZ, priority));
        LOGGER.trace("Chunk extraction requested: ({}, {}) priority={}", chunkX, chunkZ, priority);
    }

    /**
     * Request multiple chunks around a center point
     */
    public void requestChunksAround(int centerX, int centerZ, int radius) {
        // Center chunk is immediate
        requestChunk(centerX, centerZ, Priority.IMMEDIATE);

        // Adjacent chunks are high priority
        for (int dx = -1; dx <= 1; dx++) {
            for (int dz = -1; dz <= 1; dz++) {
                if (dx != 0 || dz != 0) {
                    requestChunk(centerX + dx, centerZ + dz, Priority.HIGH);
                }
            }
        }

        // Outer chunks are normal priority
        for (int dx = -radius; dx <= radius; dx++) {
            for (int dz = -radius; dz <= radius; dz++) {
                if (Math.abs(dx) > 1 || Math.abs(dz) > 1) {
                    requestChunk(centerX + dx, centerZ + dz, Priority.NORMAL);
                }
            }
        }
    }

    // =========================================================================
    // EXTRACTION
    // =========================================================================

    /**
     * Process pending chunk extractions
     * 
     * @param maxChunks Maximum chunks to process this call
     * @return Number of chunks processed
     */
    public int processQueue(int maxChunks) {
        int processed = 0;

        while (processed < maxChunks && !extractionQueue.isEmpty()) {
            ChunkRequest request = extractionQueue.poll();
            if (request != null) {
                if (extractChunk(request.chunkX, request.chunkZ)) {
                    processed++;
                }
            }
        }

        return processed;
    }

    /**
     * Extract a single chunk
     * 
     * @param chunkX Chunk X coordinate
     * @param chunkZ Chunk Z coordinate
     * @return true if extraction was successful
     */
    public boolean extractChunk(int chunkX, int chunkZ) {
        long key = packCoords(chunkX, chunkZ);

        // Mark as processing
        if (!processingChunks.add(key)) {
            return false; // Already processing
        }

        try {
            LOGGER.trace("Extracting chunk ({}, {})", chunkX, chunkZ);

            // Get buffer from pool
            ZeroCopyBuffer buffer = getBuffer();

            // In actual implementation, this would:
            // 1. Get chunk from Minecraft's world
            // 2. Extract block IDs, states, light levels
            // 3. Pack into buffer
            // 4. Send to native engine

            // Placeholder: create chunk data structure
            ChunkData chunkData = new ChunkData(chunkX, chunkZ);

            // Extract each section (16x16x16)
            for (int sectionY = 0; sectionY < 16; sectionY++) {
                if (extractSection(chunkData, sectionY, buffer)) {
                    sectionsProcessed.incrementAndGet();
                }
            }

            // Send to native engine
            sendToNative(chunkData);

            // Return buffer to pool
            returnBuffer(buffer);

            // Mark as loaded
            loadedChunks.add(key);
            chunksExtracted.incrementAndGet();

            return true;

        } catch (Exception e) {
            LOGGER.error("Failed to extract chunk ({}, {}): {}", chunkX, chunkZ, e.getMessage());
            return false;
        } finally {
            processingChunks.remove(key);
        }
    }

    /**
     * Extract a single section from a chunk
     */
    private boolean extractSection(ChunkData chunkData, int sectionY, ZeroCopyBuffer buffer) {
        // In actual implementation:
        // 1. Check if section is empty (skip if so)
        // 2. Extract block IDs using palette decoding
        // 3. Extract block states
        // 4. Extract light levels
        // 5. Store in buffer

        // Placeholder: mark section as having data
        return true;
    }

    /**
     * Send extracted chunk data to native engine
     */
    private void sendToNative(ChunkData chunkData) {
        try {
            // In actual implementation, this would call:
            // nativeBridge.submitChunk(chunkData.chunkX, chunkData.chunkZ,
            // chunkData.toBytes());

            LOGGER.trace("Sent chunk ({}, {}) to native engine",
                    chunkData.chunkX, chunkData.chunkZ);
        } catch (Exception e) {
            LOGGER.error("Failed to send chunk to native: {}", e.getMessage());
        }
    }

    // =========================================================================
    // CHUNK UPDATES
    // =========================================================================

    /**
     * Notify that a block changed in a chunk
     * 
     * @param chunkX     Chunk X
     * @param chunkZ     Chunk Z
     * @param localX     Local X (0-15)
     * @param y          World Y
     * @param localZ     Local Z (0-15)
     * @param newBlockId New block ID
     */
    public void onBlockChange(int chunkX, int chunkZ, int localX, int y, int localZ, int newBlockId) {
        long key = packCoords(chunkX, chunkZ);

        if (!loadedChunks.contains(key)) {
            return; // Chunk not loaded, ignore
        }

        try {
            // Send block update to native
            nativeBridge.setBlock(
                    chunkX * CHUNK_SIZE + localX,
                    y,
                    chunkZ * CHUNK_SIZE + localZ,
                    newBlockId);
        } catch (Exception e) {
            LOGGER.error("Failed to update block: {}", e.getMessage());
        }
    }

    /**
     * Mark a chunk as dirty (needs re-meshing)
     */
    public void markChunkDirty(int chunkX, int chunkZ) {
        long key = packCoords(chunkX, chunkZ);

        // Remove from loaded set so it will be re-extracted
        loadedChunks.remove(key);

        // Re-request with high priority
        requestChunk(chunkX, chunkZ, Priority.HIGH);
    }

    // =========================================================================
    // CHUNK UNLOADING
    // =========================================================================

    /**
     * Unload a chunk from the extractor
     */
    public void unloadChunk(int chunkX, int chunkZ) {
        long key = packCoords(chunkX, chunkZ);
        loadedChunks.remove(key);

        try {
            nativeBridge.unloadChunk(chunkX, chunkZ);
        } catch (Exception e) {
            LOGGER.error("Failed to unload chunk: {}", e.getMessage());
        }
    }

    /**
     * Unload all chunks
     */
    public void unloadAll() {
        for (Long key : loadedChunks) {
            int x = (int) (key >> 32);
            int z = (int) (key & 0xFFFFFFFFL);
            try {
                nativeBridge.unloadChunk(x, z);
            } catch (Exception ignored) {
            }
        }
        loadedChunks.clear();
        extractionQueue.clear();
    }

    // =========================================================================
    // BUFFER MANAGEMENT
    // =========================================================================

    private ZeroCopyBuffer getBuffer() {
        ZeroCopyBuffer buffer = bufferPool.poll();
        if (buffer == null) {
            buffer = ZeroCopyBuffer.allocate(32768);
        }
        return buffer;
    }

    private void returnBuffer(ZeroCopyBuffer buffer) {
        if (bufferPool.size() < 16) {
            bufferPool.offer(buffer);
        } else {
            buffer.close();
        }
    }

    // =========================================================================
    // UTILITY
    // =========================================================================

    /**
     * Pack chunk coordinates into a single long
     */
    private static long packCoords(int x, int z) {
        return ((long) x << 32) | (z & 0xFFFFFFFFL);
    }

    // =========================================================================
    // STATISTICS
    // =========================================================================

    public long getChunksExtracted() {
        return chunksExtracted.get();
    }

    public long getSectionsProcessed() {
        return sectionsProcessed.get();
    }

    public int getLoadedChunkCount() {
        return loadedChunks.size();
    }

    public int getQueueSize() {
        return extractionQueue.size();
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Chunk extraction request
     */
    private static final class ChunkRequest implements Comparable<ChunkRequest> {
        final int chunkX;
        final int chunkZ;
        final Priority priority;
        final long requestTime;

        ChunkRequest(int chunkX, int chunkZ, Priority priority) {
            this.chunkX = chunkX;
            this.chunkZ = chunkZ;
            this.priority = priority;
            this.requestTime = System.nanoTime();
        }

        @Override
        public int compareTo(ChunkRequest other) {
            int priorityCompare = priority.compareTo(other.priority);
            if (priorityCompare != 0)
                return priorityCompare;
            return Long.compare(requestTime, other.requestTime);
        }
    }

    /**
     * Extracted chunk data container
     */
    public static final class ChunkData {
        public final int chunkX;
        public final int chunkZ;

        // Section data (16 sections per chunk for 0-255 Y range)
        public final SectionData[] sections = new SectionData[16];

        public ChunkData(int chunkX, int chunkZ) {
            this.chunkX = chunkX;
            this.chunkZ = chunkZ;
        }

        /**
         * Convert to byte array for native transfer
         */
        public byte[] toBytes() {
            // In actual implementation, serialize all section data
            return new byte[0];
        }
    }

    /**
     * Single section (16x16x16) data
     */
    public static final class SectionData {
        public final int sectionY;
        public boolean isEmpty = true;

        // Block IDs (4096 entries, 16-bit each)
        public short[] blockIds;

        // Block states (4096 entries, 16-bit each)
        public short[] blockStates;

        // Light levels (4096 entries, 8-bit each: 4 block + 4 sky)
        public byte[] lightLevels;

        // Biome data (4x4x4 = 64 entries for 1.18+)
        public byte[] biomes;

        public SectionData(int sectionY) {
            this.sectionY = sectionY;
        }

        public void allocate() {
            if (blockIds == null) {
                blockIds = new short[BLOCKS_PER_SECTION];
                blockStates = new short[BLOCKS_PER_SECTION];
                lightLevels = new byte[BLOCKS_PER_SECTION];
                biomes = new byte[64];
            }
        }
    }
}
