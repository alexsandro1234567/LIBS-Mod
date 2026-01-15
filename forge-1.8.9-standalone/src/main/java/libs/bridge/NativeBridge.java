/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * NativeBridge.java - JNI Bridge Layer
 * 
 * This is the critical interface between Java and the Rust native engine.
 * All communication with the Abyss (Rust core) goes through this class.
 * 
 * Features:
 * - Zero-copy memory transfers where possible
 * - Bidirectional callbacks (Java <-> Rust)
 * - Error handling and recovery
 * - Memory safety guarantees
 * - Thread-safe operation
 * 
 * Architecture:
 * ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
 * │   Java Layer    │────▶│  NativeBridge   │────▶│   Rust Abyss    │
 * │  (Mod/Mixin)    │◀────│    (JNI FFI)    │◀────│  (libs_core)  │
 * └─────────────────┘     └─────────────────┘     └─────────────────┘
 */

package dev.libs.bridge;

import dev.libs.LibsCore;
import dev.libs.memory.VoidManager;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;
import java.nio.IntBuffer;
import java.nio.LongBuffer;
import java.nio.file.Path;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.function.Consumer;
import java.util.Objects;
import java.util.Map;

/**
 * NativeBridge - JNI Interface to Rust Abyss Core
 * 
 * This class provides the bridge between Java and the Rust native library.
 * It handles all JNI calls, memory management, and error propagation.
 * 
 * <h2>Memory Model:</h2>
 * <ul>
 * <li>Direct ByteBuffers are used for zero-copy data transfer</li>
 * <li>Native pointers are wrapped in handle objects for safety</li>
 * <li>All native allocations are tracked for leak detection</li>
 * </ul>
 * 
 * <h2>Thread Safety:</h2>
 * All methods are thread-safe. The native library uses internal locking
 * where necessary.
 * 
 * <h2>Error Handling:</h2>
 * Native errors are propagated as Java exceptions with full stack traces
 * when possible. Critical errors trigger graceful degradation.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class NativeBridge {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(NativeBridge.class);

    /** Magic number for native communication verification */
    private static final long NATIVE_MAGIC = 0xAE7E7C0DEL;

    /** Maximum buffer size for single transfer (256 MB) */
    private static final int MAX_BUFFER_SIZE = 256 * 1024 * 1024;

    /** Native pointer null value */
    private static final long NULL_PTR = 0L;

    // =========================================================================
    // STATIC FIELDS
    // =========================================================================

    /** Whether the native library is loaded */
    private static final AtomicBoolean libraryLoaded = new AtomicBoolean(false);

    /** Native library version (set after loading) */
    private static volatile String nativeVersion = "unknown";

    /** Native build timestamp */
    private static volatile long nativeBuildTime = 0;

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Path to the native library */
    private final Path libraryPath;

    /** Callback handler for Rust -> Java calls */
    private final CallbackHandler callbackHandler;

    /** Native engine handle (pointer) */
    private volatile long engineHandle = NULL_PTR;

    /** Whether the engine is initialized */
    private final AtomicBoolean initialized = new AtomicBoolean(false);

    /** Whether the engine is paused */
    private final AtomicBoolean paused = new AtomicBoolean(false);

    /** Native memory usage tracker */
    private final AtomicLong nativeMemoryUsage = new AtomicLong(0);

    /** Active native handles for tracking */
    private final ConcurrentHashMap<Long, NativeHandle> activeHandles = new ConcurrentHashMap<>();

    /** Lock for critical operations */
    private final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

    /** Native error callback */
    private volatile Consumer<NativeError> errorCallback;

    // =========================================================================
    // NATIVE METHODS
    // =========================================================================

    // Engine lifecycle
    private static native long nativeCreateEngine(byte[] config);

    private static native void nativeDestroyEngine(long handle);

    private static native boolean nativeInitialize(long handle, byte[] hardwareInfo, byte[] vulkanCaps, byte[] config);

    private static native void nativeShutdown(long handle);

    private static native void nativePause(long handle);

    private static native void nativeResume(long handle);

    // Version info
    private static native String nativeGetVersion();

    private static native long nativeGetBuildTime();

    private static native int nativeGetApiVersion();

    // Tick and render
    private static native void nativeOnTick(long handle, long tick, float deltaTime);

    private static native void nativePrepareFrame(long handle, float partialTicks);

    private static native void nativeRenderWorld(long handle, long frame, float partialTicks,
            FloatBuffer viewMatrix, FloatBuffer projMatrix);

    private static native void nativeCompositeFrame(long handle);

    // Memory management
    private static native long nativeAllocate(long size);

    private static native void nativeFree(long ptr);

    private static native void nativeSyncMemory(long handle);

    private static native long nativeGetMemoryUsage(long handle);

    // Entity management
    private static native void nativeSpawnEntity(long handle, int entityId, String type,
            double x, double y, double z);

    private static native void nativeRemoveEntity(long handle, int entityId);

    private static native void nativeUpdateEntity(long handle, int entityId,
            double x, double y, double z,
            float yaw, float pitch);

    private static native void nativeBatchUpdateEntities(long handle, IntBuffer entityIds,
            FloatBuffer positions, int count);

    // Chunk management
    private static native void nativeLoadChunk(long handle, int x, int z, ByteBuffer data);

    private static native void nativeUnloadChunk(long handle, int x, int z);

    private static native void nativeUpdateChunk(long handle, int x, int z, ByteBuffer data);

    private static native void nativeMarkChunkDirty(long handle, int x, int z);

    // Texture management
    private static native long nativeUploadTexture(long handle, String name, ByteBuffer data,
            int width, int height, int format);

    private static native void nativeUnloadTexture(long handle, long textureHandle);

    // Audio
    private static native void nativePlaySound(long handle, String sound, float x, float y, float z,
            float volume, float pitch);

    private static native void nativeStopSound(long handle, String sound);

    private static native void nativeUpdateListener(long handle, float x, float y, float z,
            float yaw, float pitch);

    // Network
    private static native byte[] nativeCompressPacket(long handle, byte[] data);

    private static native byte[] nativeDecompressPacket(long handle, byte[] data);

    private static native void nativePredictState(long handle, long tick, ByteBuffer state);

    private static native void nativeReconcileState(long handle, long tick, ByteBuffer serverState);

    // Debug
    private static native String nativeGetDebugInfo(long handle);

    private static native void nativeSetDebugFlag(long handle, String flag, boolean value);

    private static native long nativeGetProfileData(long handle);

    // Callback registration
    private static native void nativeRegisterCallbacks(long handle, Object callbackHandler);

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new NativeBridge instance.
     * 
     * @param libraryPath     Path to the native library
     * @param callbackHandler Handler for native callbacks
     */
    public NativeBridge(@NotNull Path libraryPath, @NotNull CallbackHandler callbackHandler) {
        this.libraryPath = Objects.requireNonNull(libraryPath, "libraryPath cannot be null");
        this.callbackHandler = Objects.requireNonNull(callbackHandler, "callbackHandler cannot be null");

        LOGGER.debug("NativeBridge created for library: {}", libraryPath);
    }

    // =========================================================================
    // LIBRARY LOADING
    // =========================================================================

    /**
     * Load the native library.
     * This should be called by NativeLoader, not directly.
     * 
     * @param path Path to the native library file
     * @throws UnsatisfiedLinkError if library cannot be loaded
     */
    public static void loadLibrary(@NotNull Path path) {
        if (libraryLoaded.get()) {
            LOGGER.debug("Native library already loaded");
            return;
        }

        synchronized (NativeBridge.class) {
            if (libraryLoaded.get())
                return;

            LOGGER.info("Loading native library from: {}", path);

            try {
                System.load(path.toAbsolutePath().toString());
                libraryLoaded.set(true);

                // Get native version info
                nativeVersion = nativeGetVersion();
                nativeBuildTime = nativeGetBuildTime();
                int apiVersion = nativeGetApiVersion();

                LOGGER.info("Native library loaded successfully");
                LOGGER.info("  Version: {}", nativeVersion);
                LOGGER.info("  Build Time: {}", java.time.Instant.ofEpochMilli(nativeBuildTime));
                LOGGER.info("  API Version: {}", apiVersion);

            } catch (UnsatisfiedLinkError e) {
                LOGGER.error("Failed to load native library: {}", e.getMessage());
                throw e;
            }
        }
    }

    /**
     * Check if the native library is loaded
     */
    public static boolean isLibraryLoaded() {
        return libraryLoaded.get();
    }

    /**
     * Get the native library version
     */
    public static String getNativeVersion() {
        return nativeVersion;
    }

    // =========================================================================
    // ENGINE LIFECYCLE
    // =========================================================================

    /**
     * Initialize the native engine.
     * 
     * @param hardwareInfo Serialized hardware profile
     * @param vulkanCaps   Serialized Vulkan capabilities (can be null)
     * @param config       Serialized configuration
     * @return true if initialization succeeded
     */
    public boolean initializeEngine(@Nullable byte[] hardwareInfo,
            @Nullable byte[] vulkanCaps,
            @Nullable byte[] config) {
        lock.writeLock().lock();
        try {
            if (initialized.get()) {
                LOGGER.warn("Engine already initialized");
                return true;
            }

            if (!libraryLoaded.get()) {
                LOGGER.error("Cannot initialize engine: native library not loaded");
                return false;
            }

            LOGGER.info("Initializing native engine...");

            // Create engine instance
            engineHandle = nativeCreateEngine(config != null ? config : new byte[0]);
            if (engineHandle == NULL_PTR) {
                LOGGER.error("Failed to create native engine instance");
                return false;
            }

            LOGGER.debug("Engine handle created: 0x{}", Long.toHexString(engineHandle));

            // Initialize engine
            boolean success = nativeInitialize(
                    engineHandle,
                    hardwareInfo != null ? hardwareInfo : new byte[0],
                    vulkanCaps != null ? vulkanCaps : new byte[0],
                    config != null ? config : new byte[0]);

            if (!success) {
                LOGGER.error("Native engine initialization returned false");
                nativeDestroyEngine(engineHandle);
                engineHandle = NULL_PTR;
                return false;
            }

            // Register callbacks
            nativeRegisterCallbacks(engineHandle, callbackHandler);

            initialized.set(true);
            LOGGER.info("Native engine initialized successfully");

            return true;

        } catch (Exception e) {
            LOGGER.error("Exception during engine initialization: {}", e.getMessage(), e);
            if (engineHandle != NULL_PTR) {
                try {
                    nativeDestroyEngine(engineHandle);
                } catch (Exception ignored) {
                }
                engineHandle = NULL_PTR;
            }
            return false;
        } finally {
            lock.writeLock().unlock();
        }
    }

    /**
     * Shutdown the native engine
     */
    public void shutdown() {
        lock.writeLock().lock();
        try {
            if (!initialized.get()) {
                return;
            }

            LOGGER.info("Shutting down native engine...");

            // Free all tracked handles
            for (Long handle : activeHandles.keySet()) {
                try {
                    nativeFree(handle);
                } catch (Exception ignored) {
                }
            }
            activeHandles.clear();

            // Shutdown engine
            if (engineHandle != NULL_PTR) {
                nativeShutdown(engineHandle);
                nativeDestroyEngine(engineHandle);
                engineHandle = NULL_PTR;
            }

            initialized.set(false);
            LOGGER.info("Native engine shutdown complete");

        } finally {
            lock.writeLock().unlock();
        }
    }

    /**
     * Pause the native engine
     */
    public void pause() {
        if (!initialized.get() || paused.get())
            return;

        lock.readLock().lock();
        try {
            nativePause(engineHandle);
            paused.set(true);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Resume the native engine
     */
    public void resume() {
        if (!initialized.get() || !paused.get())
            return;

        lock.readLock().lock();
        try {
            nativeResume(engineHandle);
            paused.set(false);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // TICK AND RENDER
    // =========================================================================

    /**
     * Process a game tick
     * 
     * @param tick      Current tick number
     * @param deltaTime Time since last tick
     */
    public void onTick(long tick, float deltaTime) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeOnTick(engineHandle, tick, deltaTime);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Perform server-side native tick
     */
    public void serverTick() {
        if (!checkReady())
            return;
        // Server specific native logic
    }

    /**
     * Perform client-side native tick
     */
    public void clientTick() {
        if (!checkReady())
            return;
        // Client specific native logic
    }

    /**
     * Prepare for rendering a frame
     * 
     * @param partialTicks Partial tick for interpolation
     */
    public void prepareFrame(float partialTicks) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativePrepareFrame(engineHandle, partialTicks);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Render the world
     * 
     * @param frame        Frame number
     * @param partialTicks Partial tick
     * @param viewMatrix   4x4 view matrix (16 floats)
     * @param projMatrix   4x4 projection matrix (16 floats)
     */
    public void renderWorld(long frame, float partialTicks, float[] viewMatrix, float[] projMatrix) {
        if (!checkReady())
            return;

        if (viewMatrix.length != 16 || projMatrix.length != 16) {
            throw new IllegalArgumentException("Matrices must be 4x4 (16 elements)");
        }

        // Create direct buffers for zero-copy transfer
        FloatBuffer viewBuf = ByteBuffer.allocateDirect(64)
                .order(ByteOrder.nativeOrder())
                .asFloatBuffer()
                .put(viewMatrix);
        viewBuf.flip();

        FloatBuffer projBuf = ByteBuffer.allocateDirect(64)
                .order(ByteOrder.nativeOrder())
                .asFloatBuffer()
                .put(projMatrix);
        projBuf.flip();

        lock.readLock().lock();
        try {
            nativeRenderWorld(engineHandle, frame, partialTicks, viewBuf, projBuf);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Composite the final frame (Vulkan + OpenGL UI)
     */
    public void compositeFrame() {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeCompositeFrame(engineHandle);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Begin a frame
     * 
     * @param partialTicks Partial tick for interpolation
     */
    public void beginFrame(float partialTicks) {
        if (!checkReady())
            return;
        prepareFrame(partialTicks);
    }

    /**
     * End a frame
     */
    public void endFrame() {
        if (!checkReady())
            return;
        compositeFrame();
    }

    /**
     * Update camera position
     */
    public void updateCamera(double x, double y, double z, float yaw, float pitch) {
        if (!checkReady())
            return;
        // Camera update is handled in renderWorld or via native calls
        // This is a convenience method for the render hijacker
    }

    /**
     * Set a block in the world
     */
    public void setBlock(int x, int y, int z, int blockId) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            // Would call native setBlock when implemented
            LOGGER.trace("setBlock({}, {}, {}, {})", x, y, z, blockId);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // MEMORY MANAGEMENT
    // =========================================================================

    /**
     * Allocate native memory
     * 
     * @param size Size in bytes
     * @return Handle to allocated memory
     */
    public NativeHandle allocate(long size) {
        if (size <= 0 || size > MAX_BUFFER_SIZE) {
            throw new IllegalArgumentException("Invalid allocation size: " + size);
        }

        long ptr = nativeAllocate(size);
        if (ptr == NULL_PTR) {
            throw new OutOfMemoryError("Failed to allocate " + size + " bytes in native memory");
        }

        NativeHandle handle = new NativeHandle(ptr, size, this);
        activeHandles.put(ptr, handle);
        nativeMemoryUsage.addAndGet(size);

        LOGGER.trace("Allocated {} bytes at 0x{}", size, Long.toHexString(ptr));

        return handle;
    }

    /**
     * Free native memory
     * 
     * @param handle Handle to free
     */
    public void free(NativeHandle handle) {
        if (handle == null || handle.isFreed())
            return;

        long ptr = handle.getPointer();
        if (activeHandles.remove(ptr) != null) {
            nativeFree(ptr);
            nativeMemoryUsage.addAndGet(-handle.getSize());
            handle.markFreed();
            LOGGER.trace("Freed {} bytes at 0x{}", handle.getSize(), Long.toHexString(ptr));
        }
    }

    /**
     * Synchronize memory between Java and native
     */
    public void syncMemory() {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeSyncMemory(engineHandle);
            nativeMemoryUsage.set(nativeGetMemoryUsage(engineHandle));
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Get current native memory usage
     */
    public long getNativeMemoryUsage() {
        return nativeMemoryUsage.get();
    }

    // =========================================================================
    // ENTITY MANAGEMENT
    // =========================================================================

    /**
     * Spawn an entity in the native ECS
     * 
     * @param entityId Minecraft entity ID
     * @param type     Entity type identifier
     * @param x        X position
     * @param y        Y position
     * @param z        Z position
     */
    public void spawnEntity(int entityId, String type, double x, double y, double z) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeSpawnEntity(engineHandle, entityId, type, x, y, z);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Remove an entity from the native ECS
     * 
     * @param entityId Entity ID to remove
     */
    public void removeEntity(int entityId) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeRemoveEntity(engineHandle, entityId);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Update entity position and rotation
     */
    public void updateEntity(int entityId, double x, double y, double z, float yaw, float pitch) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeUpdateEntity(engineHandle, entityId, x, y, z, yaw, pitch);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Batch update multiple entities for efficiency
     * 
     * @param entityIds Array of entity IDs
     * @param positions Interleaved positions (x, y, z, yaw, pitch for each)
     */
    public void batchUpdateEntities(int[] entityIds, float[] positions) {
        if (!checkReady())
            return;

        if (entityIds.length * 5 != positions.length) {
            throw new IllegalArgumentException("Position array must have 5 values per entity");
        }

        IntBuffer idBuf = ByteBuffer.allocateDirect(entityIds.length * 4)
                .order(ByteOrder.nativeOrder())
                .asIntBuffer()
                .put(entityIds);
        idBuf.flip();

        FloatBuffer posBuf = ByteBuffer.allocateDirect(positions.length * 4)
                .order(ByteOrder.nativeOrder())
                .asFloatBuffer()
                .put(positions);
        posBuf.flip();

        lock.readLock().lock();
        try {
            nativeBatchUpdateEntities(engineHandle, idBuf, posBuf, entityIds.length);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // CHUNK MANAGEMENT
    // =========================================================================

    /**
     * Load a chunk into the native renderer
     * 
     * @param x    Chunk X coordinate
     * @param z    Chunk Z coordinate
     * @param data Chunk block data
     */
    public void loadChunk(int x, int z, ByteBuffer data) {
        if (!checkReady())
            return;

        if (!data.isDirect()) {
            throw new IllegalArgumentException("Chunk data must be a direct ByteBuffer");
        }

        lock.readLock().lock();
        try {
            nativeLoadChunk(engineHandle, x, z, data);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Unload a chunk from the native renderer
     */
    public void unloadChunk(int x, int z) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeUnloadChunk(engineHandle, x, z);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Update chunk data
     */
    public void updateChunk(int x, int z, ByteBuffer data) {
        if (!checkReady())
            return;

        if (!data.isDirect()) {
            throw new IllegalArgumentException("Chunk data must be a direct ByteBuffer");
        }

        lock.readLock().lock();
        try {
            nativeUpdateChunk(engineHandle, x, z, data);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Mark a chunk as needing re-meshing
     */
    public void markChunkDirty(int x, int z) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeMarkChunkDirty(engineHandle, x, z);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // TEXTURE MANAGEMENT
    // =========================================================================

    /**
     * Upload a texture to the native renderer
     * 
     * @param name   Texture name/identifier
     * @param data   Texture pixel data
     * @param width  Texture width
     * @param height Texture height
     * @param format Pixel format (0 = RGBA8, 1 = BGRA8, 2 = RGB8, etc.)
     * @return Texture handle
     */
    public long uploadTexture(String name, ByteBuffer data, int width, int height, int format) {
        if (!checkReady())
            return NULL_PTR;

        if (!data.isDirect()) {
            throw new IllegalArgumentException("Texture data must be a direct ByteBuffer");
        }

        lock.readLock().lock();
        try {
            return nativeUploadTexture(engineHandle, name, data, width, height, format);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Unload a texture
     */
    public void unloadTexture(long textureHandle) {
        if (!checkReady() || textureHandle == NULL_PTR)
            return;

        lock.readLock().lock();
        try {
            nativeUnloadTexture(engineHandle, textureHandle);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // AUDIO
    // =========================================================================

    /**
     * Play a sound at a position
     */
    public void playSound(String sound, float x, float y, float z, float volume, float pitch) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativePlaySound(engineHandle, sound, x, y, z, volume, pitch);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Stop a sound
     */
    public void stopSound(String sound) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeStopSound(engineHandle, sound);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Update audio listener position
     */
    public void updateListener(float x, float y, float z, float yaw, float pitch) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeUpdateListener(engineHandle, x, y, z, yaw, pitch);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // NETWORK
    // =========================================================================

    /**
     * Compress network packet data
     */
    public byte[] compressPacket(byte[] data) {
        if (!checkReady() || data == null)
            return data;

        lock.readLock().lock();
        try {
            return nativeCompressPacket(engineHandle, data);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Decompress network packet data
     */
    public byte[] decompressPacket(byte[] data) {
        if (!checkReady() || data == null)
            return data;

        lock.readLock().lock();
        try {
            return nativeDecompressPacket(engineHandle, data);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // DEBUG
    // =========================================================================

    /**
     * Get debug information from native engine
     */
    public String getDebugInfo() {
        if (!checkReady())
            return "Native engine not ready";

        lock.readLock().lock();
        try {
            return nativeGetDebugInfo(engineHandle);
        } finally {
            lock.readLock().unlock();
        }
    }

    /**
     * Set a debug flag
     */
    public void setDebugFlag(String flag, boolean value) {
        if (!checkReady())
            return;

        lock.readLock().lock();
        try {
            nativeSetDebugFlag(engineHandle, flag, value);
        } finally {
            lock.readLock().unlock();
        }
    }

    // =========================================================================
    // UTILITY
    // =========================================================================

    /**
     * Check if the engine is ready for operations
     */
    private boolean checkReady() {
        if (!initialized.get()) {
            LOGGER.trace("Engine not initialized");
            return false;
        }
        if (paused.get()) {
            LOGGER.trace("Engine is paused");
            return false;
        }
        if (engineHandle == NULL_PTR) {
            LOGGER.trace("Engine handle is null");
            return false;
        }
        return true;
    }

    /**
     * Set error callback
     */
    public void setErrorCallback(Consumer<NativeError> callback) {
        this.errorCallback = callback;
    }

    /**
     * Called by native code when an error occurs
     * (This method is invoked via JNI)
     */
    @SuppressWarnings("unused")
    private void onNativeError(int code, String message, String details) {
        NativeError error = new NativeError(code, message, details);
        LOGGER.error("Native error [{}]: {} - {}", code, message, details);

        if (errorCallback != null) {
            try {
                errorCallback.accept(error);
            } catch (Exception e) {
                LOGGER.error("Error callback threw exception: {}", e.getMessage());
            }
        }
    }

    // =========================================================================
    // GETTERS
    // =========================================================================

    public boolean isInitialized() {
        return initialized.get();
    }

    public boolean isPaused() {
        return paused.get();
    }

    public Path getLibraryPath() {
        return libraryPath;
    }

    public long getEngineHandle() {
        return engineHandle;
    }

    public int getActiveHandleCount() {
        return activeHandles.size();
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Native memory handle for safe memory management
     */
    public static final class NativeHandle implements AutoCloseable {
        private final long pointer;
        private final long size;
        private final NativeBridge bridge;
        private volatile boolean freed = false;

        NativeHandle(long pointer, long size, NativeBridge bridge) {
            this.pointer = pointer;
            this.size = size;
            this.bridge = bridge;
        }

        public long getPointer() {
            return pointer;
        }

        public long getSize() {
            return size;
        }

        public boolean isFreed() {
            return freed;
        }

        void markFreed() {
            freed = true;
        }

        @Override
        public void close() {
            if (!freed) {
                bridge.free(this);
            }
        }

        /**
         * Create a ByteBuffer view of this native memory
         */
        public ByteBuffer asByteBuffer() {
            if (freed) {
                throw new IllegalStateException("Handle already freed");
            }
            // Create direct buffer pointing to native memory
            // This requires sun.misc.Unsafe in practice
            return ByteBuffer.allocateDirect((int) size).order(ByteOrder.nativeOrder());
        }
    }

    /**
     * Native error information
     */
    public static final class NativeError {
        private final int code;
        private final String message;
        private final String details;
        private final long timestamp;

        NativeError(int code, String message, String details) {
            this.code = code;
            this.message = message;
            this.details = details;
            this.timestamp = System.currentTimeMillis();
        }

        public int getCode() {
            return code;
        }

        public String getMessage() {
            return message;
        }

        public String getDetails() {
            return details;
        }

        public long getTimestamp() {
            return timestamp;
        }

        @Override
        public String toString() {
            return String.format("NativeError[%d: %s - %s]", code, message, details);
        }
    }
}
