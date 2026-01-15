/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * CallbackHandler.java - Rust to Java Callback System
 * 
 * Handles callbacks from the native Rust engine back to Java.
 * Used for events, errors, and async operation completions.
 */

package dev.libs.bridge;

import dev.libs.LibsCore;
import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Consumer;
import java.util.Map;
import java.util.Objects;

/**
 * CallbackHandler - Native to Java Callback System
 * 
 * This class receives callbacks from the Rust native engine and
 * dispatches them to the appropriate handlers in Java.
 * 
 * <h2>Callback Types:</h2>
 * <ul>
 * <li>Error callbacks - Native error reporting</li>
 * <li>Render callbacks - Frame ready notifications</li>
 * <li>Chunk callbacks - Chunk processing complete</li>
 * <li>Entity callbacks - Entity state updates</li>
 * <li>Audio callbacks - Sound completion events</li>
 * <li>Network callbacks - Packet processing</li>
 * </ul>
 * 
 * <h2>Thread Safety:</h2>
 * Callbacks may be invoked from any thread. Handlers should be
 * thread-safe or use the provided executor for dispatch.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class CallbackHandler {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(CallbackHandler.class);

    /** Callback types */
    public static final int CALLBACK_ERROR = 0;
    public static final int CALLBACK_LOG = 1;
    public static final int CALLBACK_FRAME_READY = 10;
    public static final int CALLBACK_CHUNK_READY = 20;
    public static final int CALLBACK_CHUNK_MESH_READY = 21;
    public static final int CALLBACK_ENTITY_UPDATE = 30;
    public static final int CALLBACK_ENTITY_REMOVED = 31;
    public static final int CALLBACK_SOUND_COMPLETE = 40;
    public static final int CALLBACK_NETWORK_PACKET = 50;
    public static final int CALLBACK_MEMORY_WARNING = 60;
    public static final int CALLBACK_MEMORY_CRITICAL = 61;
    public static final int CALLBACK_PROFILING_DATA = 70;

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Reference to the core */
    private final LibsCore core;

    /** Callback counter for statistics */
    private final AtomicLong callbackCounter = new AtomicLong(0);

    /** Executor for async callback dispatch */
    private final ExecutorService callbackExecutor;

    /** Registered callback handlers by type */
    private final Map<Integer, Consumer<CallbackData>> handlers;

    /** Generic error handler */
    private volatile Consumer<NativeError> errorHandler;

    /** Log message handler */
    private volatile Consumer<LogMessage> logHandler;

    /** Frame ready handler */
    private volatile Runnable frameReadyHandler;

    /** Chunk ready handler */
    private volatile ChunkReadyHandler chunkReadyHandler;

    /** Entity update handler */
    private volatile EntityUpdateHandler entityUpdateHandler;

    /** Memory warning handler */
    private volatile Consumer<MemoryWarning> memoryWarningHandler;

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new CallbackHandler
     * 
     * @param core Reference to the LibsCore instance
     */
    public CallbackHandler(@NotNull LibsCore core) {
        this.core = Objects.requireNonNull(core, "core cannot be null");
        this.handlers = new ConcurrentHashMap<>();

        this.callbackExecutor = Executors.newFixedThreadPool(2, r -> {
            Thread t = new Thread(r, "Libs-Callback-" + System.nanoTime());
            t.setDaemon(true);
            t.setPriority(Thread.NORM_PRIORITY + 1);
            return t;
        });

        // Register default handlers
        setupDefaultHandlers();

        LOGGER.debug("CallbackHandler initialized");
    }

    // =========================================================================
    // DEFAULT HANDLERS
    // =========================================================================

    /**
     * Setup default callback handlers
     */
    private void setupDefaultHandlers() {
        // Error handler
        handlers.put(CALLBACK_ERROR, data -> {
            if (errorHandler != null) {
                NativeError error = new NativeError(
                        data.getInt("code", 0),
                        data.getString("message", "Unknown error"),
                        data.getString("details", ""));
                errorHandler.accept(error);
            }
        });

        // Log handler
        handlers.put(CALLBACK_LOG, data -> {
            if (logHandler != null) {
                LogMessage msg = new LogMessage(
                        data.getInt("level", 0),
                        data.getString("message", ""),
                        data.getString("module", "native"));
                logHandler.accept(msg);
            } else {
                // Default log handling
                int level = data.getInt("level", 0);
                String message = data.getString("message", "");
                switch (level) {
                    case 0:
                        LOGGER.trace("[Native] {}", message);
                        break;
                    case 1:
                        LOGGER.debug("[Native] {}", message);
                        break;
                    case 2:
                        LOGGER.info("[Native] {}", message);
                        break;
                    case 3:
                        LOGGER.warn("[Native] {}", message);
                        break;
                    case 4:
                        LOGGER.error("[Native] {}", message);
                        break;
                    default:
                        LOGGER.info("[Native] {}", message);
                }
            }
        });

        // Frame ready handler
        handlers.put(CALLBACK_FRAME_READY, data -> {
            if (frameReadyHandler != null) {
                frameReadyHandler.run();
            }
        });

        // Chunk ready handler
        handlers.put(CALLBACK_CHUNK_READY, data -> {
            if (chunkReadyHandler != null) {
                chunkReadyHandler.onChunkReady(
                        data.getInt("x", 0),
                        data.getInt("z", 0),
                        data.getLong("handle", 0));
            }
        });

        // Chunk mesh ready handler
        handlers.put(CALLBACK_CHUNK_MESH_READY, data -> {
            if (chunkReadyHandler != null) {
                chunkReadyHandler.onChunkMeshReady(
                        data.getInt("x", 0),
                        data.getInt("z", 0),
                        data.getInt("vertices", 0),
                        data.getInt("indices", 0));
            }
        });

        // Entity update handler
        handlers.put(CALLBACK_ENTITY_UPDATE, data -> {
            if (entityUpdateHandler != null) {
                entityUpdateHandler.onEntityUpdate(
                        data.getInt("id", 0),
                        data.getDouble("x", 0),
                        data.getDouble("y", 0),
                        data.getDouble("z", 0),
                        data.getFloat("yaw", 0),
                        data.getFloat("pitch", 0));
            }
        });

        // Entity removed handler
        handlers.put(CALLBACK_ENTITY_REMOVED, data -> {
            if (entityUpdateHandler != null) {
                entityUpdateHandler.onEntityRemoved(data.getInt("id", 0));
            }
        });

        // Memory warning
        handlers.put(CALLBACK_MEMORY_WARNING, data -> {
            if (memoryWarningHandler != null) {
                memoryWarningHandler.accept(new MemoryWarning(
                        MemoryWarning.Level.WARNING,
                        data.getLong("used", 0),
                        data.getLong("max", 0),
                        data.getString("message", "")));
            } else {
                LOGGER.warn("Memory warning: {} (Used: {} / Max: {})",
                        data.getString("message", ""),
                        data.getLong("used", 0),
                        data.getLong("max", 0));
            }
        });

        // Memory critical
        handlers.put(CALLBACK_MEMORY_CRITICAL, data -> {
            if (memoryWarningHandler != null) {
                memoryWarningHandler.accept(new MemoryWarning(
                        MemoryWarning.Level.CRITICAL,
                        data.getLong("used", 0),
                        data.getLong("max", 0),
                        data.getString("message", "")));
            } else {
                LOGGER.error("CRITICAL MEMORY WARNING: {} (Used: {} / Max: {})",
                        data.getString("message", ""),
                        data.getLong("used", 0),
                        data.getLong("max", 0));
            }
        });
    }

    // =========================================================================
    // CALLBACK ENTRY POINTS (Called from native code via JNI)
    // =========================================================================

    /**
     * Main callback entry point from native code.
     * This method is invoked via JNI.
     * 
     * @param type Callback type
     * @param data Callback data as key-value pairs
     */
    @SuppressWarnings("unused")
    public void onCallback(int type, Object[] data) {
        callbackCounter.incrementAndGet();

        try {
            CallbackData callbackData = new CallbackData(data);

            Consumer<CallbackData> handler = handlers.get(type);
            if (handler != null) {
                callbackExecutor.execute(() -> {
                    try {
                        handler.accept(callbackData);
                    } catch (Exception e) {
                        LOGGER.error("Callback handler error for type {}: {}", type, e.getMessage(), e);
                    }
                });
            } else {
                LOGGER.debug("No handler for callback type: {}", type);
            }
        } catch (Exception e) {
            LOGGER.error("Error processing callback type {}: {}", type, e.getMessage(), e);
        }
    }

    /**
     * Simple callback with integer data.
     * This method is invoked via JNI for simple callbacks.
     */
    @SuppressWarnings("unused")
    public void onSimpleCallback(int type, int value) {
        callbackCounter.incrementAndGet();

        switch (type) {
            case CALLBACK_FRAME_READY:
                if (frameReadyHandler != null) {
                    callbackExecutor.execute(frameReadyHandler);
                }
                break;
            case CALLBACK_ENTITY_REMOVED:
                if (entityUpdateHandler != null) {
                    final int id = value;
                    callbackExecutor.execute(() -> entityUpdateHandler.onEntityRemoved(id));
                }
                break;
            default:
                LOGGER.trace("Simple callback type {} with value {}", type, value);
        }
    }

    /**
     * Error callback from native code.
     * This method is invoked via JNI.
     */
    @SuppressWarnings("unused")
    public void onError(int code, String message, String details) {
        callbackCounter.incrementAndGet();

        LOGGER.error("Native error [{}]: {} - {}", code, message, details);

        if (errorHandler != null) {
            NativeError error = new NativeError(code, message, details);
            callbackExecutor.execute(() -> errorHandler.accept(error));
        }
    }

    /**
     * Log callback from native code.
     * This method is invoked via JNI.
     */
    @SuppressWarnings("unused")
    public void onLog(int level, String module, String message) {
        callbackCounter.incrementAndGet();

        if (logHandler != null) {
            LogMessage msg = new LogMessage(level, message, module);
            callbackExecutor.execute(() -> logHandler.accept(msg));
        } else {
            // Direct logging
            String formatted = String.format("[%s] %s", module, message);
            switch (level) {
                case 0:
                    LOGGER.trace(formatted);
                    break;
                case 1:
                    LOGGER.debug(formatted);
                    break;
                case 2:
                    LOGGER.info(formatted);
                    break;
                case 3:
                    LOGGER.warn(formatted);
                    break;
                case 4:
                    LOGGER.error(formatted);
                    break;
                default:
                    LOGGER.info(formatted);
            }
        }
    }

    /**
     * Chunk completion callback from native code.
     * This method is invoked via JNI.
     */
    @SuppressWarnings("unused")
    public void onChunkComplete(int x, int z, long handle, int vertices, int indices) {
        callbackCounter.incrementAndGet();

        if (chunkReadyHandler != null) {
            callbackExecutor.execute(() -> {
                chunkReadyHandler.onChunkReady(x, z, handle);
                chunkReadyHandler.onChunkMeshReady(x, z, vertices, indices);
            });
        }
    }

    /**
     * Entity batch update callback from native code.
     * This method is invoked via JNI.
     */
    @SuppressWarnings("unused")
    public void onEntityBatch(int[] ids, double[] positions) {
        callbackCounter.incrementAndGet();

        if (entityUpdateHandler != null) {
            callbackExecutor.execute(() -> {
                for (int i = 0; i < ids.length; i++) {
                    int idx = i * 5;
                    if (idx + 4 < positions.length) {
                        entityUpdateHandler.onEntityUpdate(
                                ids[i],
                                positions[idx],
                                positions[idx + 1],
                                positions[idx + 2],
                                (float) positions[idx + 3],
                                (float) positions[idx + 4]);
                    }
                }
            });
        }
    }

    // =========================================================================
    // HANDLER REGISTRATION
    // =========================================================================

    /**
     * Register a custom callback handler
     */
    public void registerHandler(int type, Consumer<CallbackData> handler) {
        handlers.put(type, handler);
    }

    /**
     * Remove a callback handler
     */
    public void removeHandler(int type) {
        handlers.remove(type);
    }

    /**
     * Set error handler
     */
    public void setErrorHandler(Consumer<NativeError> handler) {
        this.errorHandler = handler;
    }

    /**
     * Set log handler
     */
    public void setLogHandler(Consumer<LogMessage> handler) {
        this.logHandler = handler;
    }

    /**
     * Set frame ready handler
     */
    public void setFrameReadyHandler(Runnable handler) {
        this.frameReadyHandler = handler;
    }

    /**
     * Set chunk ready handler
     */
    public void setChunkReadyHandler(ChunkReadyHandler handler) {
        this.chunkReadyHandler = handler;
    }

    /**
     * Set entity update handler
     */
    public void setEntityUpdateHandler(EntityUpdateHandler handler) {
        this.entityUpdateHandler = handler;
    }

    /**
     * Set memory warning handler
     */
    public void setMemoryWarningHandler(Consumer<MemoryWarning> handler) {
        this.memoryWarningHandler = handler;
    }

    // =========================================================================
    // LIFECYCLE
    // =========================================================================

    /**
     * Shutdown the callback handler
     */
    public void shutdown() {
        LOGGER.info("Shutting down CallbackHandler (processed {} callbacks)", callbackCounter.get());

        callbackExecutor.shutdown();
        handlers.clear();

        errorHandler = null;
        logHandler = null;
        frameReadyHandler = null;
        chunkReadyHandler = null;
        entityUpdateHandler = null;
        memoryWarningHandler = null;
    }

    // =========================================================================
    // STATISTICS
    // =========================================================================

    /**
     * Get total callback count
     */
    public long getCallbackCount() {
        return callbackCounter.get();
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Callback data wrapper for key-value access
     */
    public static final class CallbackData {
        private final Map<String, Object> data;

        CallbackData(Object[] rawData) {
            this.data = new ConcurrentHashMap<>();

            if (rawData != null) {
                for (int i = 0; i < rawData.length - 1; i += 2) {
                    if (rawData[i] instanceof String) {
                        data.put((String) rawData[i], rawData[i + 1]);
                    }
                }
            }
        }

        public Object get(String key) {
            return data.get(key);
        }

        public String getString(String key, String defaultValue) {
            Object val = data.get(key);
            return val instanceof String ? (String) val : defaultValue;
        }

        public int getInt(String key, int defaultValue) {
            Object val = data.get(key);
            if (val instanceof Number) {
                return ((Number) val).intValue();
            }
            return defaultValue;
        }

        public long getLong(String key, long defaultValue) {
            Object val = data.get(key);
            if (val instanceof Number) {
                return ((Number) val).longValue();
            }
            return defaultValue;
        }

        public float getFloat(String key, float defaultValue) {
            Object val = data.get(key);
            if (val instanceof Number) {
                return ((Number) val).floatValue();
            }
            return defaultValue;
        }

        public double getDouble(String key, double defaultValue) {
            Object val = data.get(key);
            if (val instanceof Number) {
                return ((Number) val).doubleValue();
            }
            return defaultValue;
        }

        public boolean getBoolean(String key, boolean defaultValue) {
            Object val = data.get(key);
            if (val instanceof Boolean) {
                return (Boolean) val;
            }
            return defaultValue;
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
            return String.format("NativeError[%d: %s]", code, message);
        }
    }

    /**
     * Log message from native code
     */
    public static final class LogMessage {
        public enum Level {
            TRACE, DEBUG, INFO, WARN, ERROR
        }

        private final Level level;
        private final String message;
        private final String module;
        private final long timestamp;

        LogMessage(int levelOrdinal, String message, String module) {
            Level[] levels = Level.values();
            this.level = levelOrdinal >= 0 && levelOrdinal < levels.length
                    ? levels[levelOrdinal]
                    : Level.INFO;
            this.message = message;
            this.module = module;
            this.timestamp = System.currentTimeMillis();
        }

        public Level getLevel() {
            return level;
        }

        public String getMessage() {
            return message;
        }

        public String getModule() {
            return module;
        }

        public long getTimestamp() {
            return timestamp;
        }
    }

    /**
     * Memory warning information
     */
    public static final class MemoryWarning {
        public enum Level {
            WARNING, CRITICAL
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
            return maxBytes > 0 ? (float) usedBytes / maxBytes * 100 : 0;
        }

        public String getMessage() {
            return message;
        }
    }

    /**
     * Interface for chunk ready callbacks
     */
    public interface ChunkReadyHandler {
        void onChunkReady(int x, int z, long handle);

        void onChunkMeshReady(int x, int z, int vertices, int indices);
    }

    /**
     * Interface for entity update callbacks
     */
    public interface EntityUpdateHandler {
        void onEntityUpdate(int entityId, double x, double y, double z, float yaw, float pitch);

        void onEntityRemoved(int entityId);
    }
}
