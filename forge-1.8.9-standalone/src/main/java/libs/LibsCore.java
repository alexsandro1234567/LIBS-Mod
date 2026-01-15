/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Libs Team
 * 
 * LibsCore.java - Main Entry Point and Lifecycle Manager
 * 
 * This is the heart of LIBS. It manages:
 * - Native library loading and initialization
 * - Hardware detection and profiling
 * - Module lifecycle (render, physics, audio, network)
 * - Integration with Minecraft's mod loaders
 * 
 * Architecture:
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                         MINECRAFT PROCESS                           │
 * ├─────────────────────────────────────────────────────────────────────┤
 * │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐ │
 * │  │   LibsCore    │──│   NativeBridge  │──│   Rust Abyss Core   │ │
 * │  │   (This Class)  │  │   (JNI Layer)   │  │   (libs_core.dll) │ │
 * │  └─────────────────┘  └─────────────────┘  └─────────────────────┘ │
 * │           │                    │                      │            │
 * │           ▼                    ▼                      ▼            │
 * │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐ │
 * │  │  Mod Loaders    │  │  Zero-Copy Mem  │  │   Vulkan Renderer   │ │
 * │  │ (Forge/Fabric)  │  │   Management    │  │   ECS / Physics     │ │
 * │  └─────────────────┘  └─────────────────┘  └─────────────────────┘ │
 * └─────────────────────────────────────────────────────────────────────┘
 */

package dev.libs;

import dev.libs.bridge.NativeBridge;
import dev.libs.bridge.CallbackHandler;
import dev.libs.hardware.HardwareDetector;
import dev.libs.hardware.HardwareProfile;
import dev.libs.hardware.VulkanCapabilities;
import dev.libs.loader.NativeLoader;
import dev.libs.loader.NativeExtractionResult;
import dev.libs.memory.VoidManager;
import dev.libs.memory.AssetInterceptor;
import dev.libs.network.PredictiveNetcode;
import dev.libs.render.RenderHijacker;
import dev.libs.world.ChunkDataExtractor;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Consumer;
import java.util.function.Supplier;
import java.util.Map;
import java.util.List;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Properties;
import java.util.Objects;

/**
 * LibsCore - The Universal Monolith Entry Point
 * 
 * This class serves as the main entry point for LIBS.
 * It initializes all subsystems and manages the lifecycle of the
 * hybrid Java/Rust engine.
 * 
 * <h2>Initialization Order:</h2>
 * <ol>
 * <li>Configuration loading</li>
 * <li>Hardware detection</li>
 * <li>Native library extraction and loading</li>
 * <li>Native engine initialization</li>
 * <li>Subsystem startup (render, physics, audio, network)</li>
 * <li>Minecraft integration hooks</li>
 * </ol>
 * 
 * <h2>Thread Safety:</h2>
 * This class is designed to be thread-safe. All state mutations
 * are protected by atomic operations or synchronized blocks.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class LibsCore {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    /** The mod identifier */
    public static final String MOD_ID = "libs";

    /** The mod display name */
    public static final String MOD_NAME = "LIBS";

    /** Current version (semantic versioning) */
    public static final String VERSION = "1.0.0-alpha";

    /** Build number for this release */
    public static final int BUILD_NUMBER = 1;

    /** Minimum supported Minecraft version */
    public static final String MIN_MC_VERSION = "1.8.9";

    /** Maximum supported Minecraft version */
    public static final String MAX_MC_VERSION = "1.21.1";

    /** Native library base name (without extension) */
    public static final String NATIVE_LIB_NAME = "libs_core";

    /** Initialization timeout in milliseconds */
    private static final long INIT_TIMEOUT_MS = 30_000;

    /** Native sync interval for memory garbage collection */
    private static final long NATIVE_GC_INTERVAL_MS = 5_000;

    /** Maximum retry attempts for native loading */
    private static final int MAX_NATIVE_LOAD_RETRIES = 3;

    // =========================================================================
    // STATIC FIELDS
    // =========================================================================

    /** Logger instance */
    private static final Logger LOGGER = LogManager.getLogger(LibsCore.class);

    /** Singleton instance */
    private static volatile LibsCore instance;

    /** Lock for singleton initialization */
    private static final Object INSTANCE_LOCK = new Object();

    /** Initialization state */
    private static final AtomicReference<InitState> initState = new AtomicReference<>(InitState.NOT_STARTED);

    /** Initialization error, if any */
    private static volatile Throwable initError;

    // =========================================================================
    // ENUMS
    // =========================================================================

    /**
     * Initialization state enumeration
     */
    public enum InitState {
        /** Initialization has not started */
        NOT_STARTED,
        /** Currently loading configuration */
        LOADING_CONFIG,
        /** Detecting hardware capabilities */
        DETECTING_HARDWARE,
        /** Extracting native libraries */
        EXTRACTING_NATIVES,
        /** Loading native libraries */
        LOADING_NATIVES,
        /** Initializing native engine */
        INITIALIZING_ENGINE,
        /** Starting subsystems */
        STARTING_SUBSYSTEMS,
        /** Hooking into Minecraft */
        HOOKING_MINECRAFT,
        /** Fully initialized and running */
        RUNNING,
        /** Shutting down */
        SHUTTING_DOWN,
        /** Shutdown complete */
        SHUTDOWN,
        /** Initialization failed */
        FAILED
    }

    /**
     * Engine operation mode
     */
    public enum OperationMode {
        /** Full Vulkan rendering mode */
        VULKAN_FULL,
        /** Hybrid mode (Vulkan world, OpenGL UI) */
        VULKAN_HYBRID,
        /** OpenGL fallback mode */
        OPENGL_FALLBACK,
        /** Safe mode (minimal hooks) */
        SAFE_MODE,
        /** Disabled (passthrough) */
        DISABLED
    }

    /**
     * Subsystem status
     */
    public enum SubsystemStatus {
        /** Not initialized */
        UNINITIALIZED,
        /** Starting up */
        STARTING,
        /** Running normally */
        RUNNING,
        /** Running in degraded mode */
        DEGRADED,
        /** Stopped */
        STOPPED,
        /** Failed to start */
        FAILED
    }

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Configuration instance */
    private final LibsConfig config;

    /** Hardware profile */
    private final HardwareProfile hardwareProfile;

    /** Vulkan capabilities */
    private final VulkanCapabilities vulkanCaps;

    /** Native bridge instance */
    private final NativeBridge nativeBridge;

    /** Callback handler for Rust -> Java calls */
    private final CallbackHandler callbackHandler;

    /** Memory manager (Void Manager) */
    private final VoidManager voidManager;

    /** Asset interceptor */
    private final AssetInterceptor assetInterceptor;

    /** Render hijacker */
    private final RenderHijacker renderHijacker;

    /** Chunk data extractor */
    private final ChunkDataExtractor chunkExtractor;

    /** Predictive netcode */
    private final PredictiveNetcode netcode;

    /** Current operation mode */
    private volatile OperationMode operationMode;

    /** Subsystem status map */
    private final ConcurrentHashMap<String, SubsystemStatus> subsystemStatus;

    /** Main executor service */
    private final ExecutorService mainExecutor;

    /** Scheduled executor for periodic tasks */
    private final ScheduledExecutorService scheduledExecutor;

    /** Statistics tracking */
    private final LibsStatistics statistics;

    /** Event listeners */
    private final List<LibsEventListener> eventListeners;

    /** Native library path */
    private final Path nativeLibPath;

    /** Startup timestamp */
    private final long startupTimestamp;

    /** Frame counter */
    private final AtomicLong frameCounter;

    /** Tick counter */
    private final AtomicLong tickCounter;

    /** Running flag */
    private final AtomicBoolean running;

    /** Paused flag */
    private final AtomicBoolean paused;

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Private constructor - use {@link #initialize()} to create instance
     */
    private LibsCore(
            LibsConfig config,
            HardwareProfile hardwareProfile,
            VulkanCapabilities vulkanCaps,
            Path nativeLibPath) {
        this.startupTimestamp = System.currentTimeMillis();
        this.config = Objects.requireNonNull(config, "config cannot be null");
        this.hardwareProfile = Objects.requireNonNull(hardwareProfile, "hardwareProfile cannot be null");
        this.vulkanCaps = vulkanCaps; // Can be null if Vulkan not available
        this.nativeLibPath = Objects.requireNonNull(nativeLibPath, "nativeLibPath cannot be null");

        // Initialize atomic state
        this.frameCounter = new AtomicLong(0);
        this.tickCounter = new AtomicLong(0);
        this.running = new AtomicBoolean(false);
        this.paused = new AtomicBoolean(false);

        // Initialize subsystem tracking
        this.subsystemStatus = new ConcurrentHashMap<>();
        this.eventListeners = Collections.synchronizedList(new ArrayList<>());

        // Initialize executors
        int threadCount = Math.max(2, Runtime.getRuntime().availableProcessors() - 2);
        this.mainExecutor = Executors.newFixedThreadPool(threadCount, r -> {
            Thread t = new Thread(r, "Libs-Worker-" + System.nanoTime());
            t.setDaemon(true);
            t.setPriority(Thread.NORM_PRIORITY + 1);
            return t;
        });

        this.scheduledExecutor = Executors.newScheduledThreadPool(2, r -> {
            Thread t = new Thread(r, "Libs-Scheduler-" + System.nanoTime());
            t.setDaemon(true);
            return t;
        });

        // Initialize statistics
        this.statistics = new LibsStatistics();

        // Determine operation mode
        this.operationMode = determineOperationMode();

        // Initialize subsystems (but don't start yet)
        this.callbackHandler = new CallbackHandler(this);
        this.nativeBridge = new NativeBridge(nativeLibPath, callbackHandler);
        this.voidManager = new VoidManager(config.getMemoryConfig());
        this.assetInterceptor = new AssetInterceptor(voidManager);
        this.renderHijacker = new RenderHijacker(config.getRenderConfig(), nativeBridge);
        this.chunkExtractor = new ChunkDataExtractor(nativeBridge);
        this.netcode = new PredictiveNetcode();

        LOGGER.info("LibsCore instance created in {} mode", operationMode);
    }

    // =========================================================================
    // STATIC INITIALIZATION
    // =========================================================================

    /**
     * Initialize the Libs engine.
     * 
     * This method is idempotent - calling it multiple times will
     * return the same instance after the first successful initialization.
     * 
     * @return CompletableFuture that completes when initialization is done
     */
    public static CompletableFuture<LibsCore> initialize() {
        return initialize(null);
    }

    /**
     * Initialize the Libs engine with custom configuration path.
     * 
     * @param configPath Path to configuration file, or null for default
     * @return CompletableFuture that completes when initialization is done
     */
    public static CompletableFuture<LibsCore> initialize(@Nullable Path configPath) {
        // Check if already initialized
        if (instance != null && initState.get() == InitState.RUNNING) {
            return CompletableFuture.completedFuture(instance);
        }

        // Check if initialization failed
        if (initState.get() == InitState.FAILED) {
            CompletableFuture<LibsCore> failed = new CompletableFuture<>();
            failed.completeExceptionally(new LibsInitializationException(
                    "Previous initialization failed", initError));
            return failed;
        }

        return CompletableFuture.supplyAsync(() -> {
            synchronized (INSTANCE_LOCK) {
                // Double-check after acquiring lock
                if (instance != null && initState.get() == InitState.RUNNING) {
                    return instance;
                }

                try {
                    instance = performInitialization(configPath);
                    return instance;
                } catch (Exception e) {
                    initState.set(InitState.FAILED);
                    initError = e;
                    throw new LibsInitializationException("Initialization failed", e);
                }
            }
        });
    }

    /**
     * Perform the actual initialization sequence
     */
    private static LibsCore performInitialization(@Nullable Path configPath) throws Exception {
        long startTime = System.currentTimeMillis();

        LOGGER.info("╔══════════════════════════════════════════════════════════════════╗");
        LOGGER.info("║                    LIBS - GENESIS                       ║");
        LOGGER.info("║                        Alpha v{} Initializing                    ║", VERSION);
        LOGGER.info("╚══════════════════════════════════════════════════════════════════╝");

        // Step 1: Load configuration
        initState.set(InitState.LOADING_CONFIG);
        LOGGER.info("[1/7] Loading configuration...");
        LibsConfig config = loadConfiguration(configPath);

        // Step 2: Detect hardware
        initState.set(InitState.DETECTING_HARDWARE);
        LOGGER.info("[2/7] Detecting hardware...");
        HardwareDetector detector = new HardwareDetector();
        HardwareProfile hardwareProfile = detector.detect();
        LOGGER.info("  CPU: {} ({} cores)", hardwareProfile.getCpuName(), hardwareProfile.getCpuCores());
        LOGGER.info("  GPU: {}", hardwareProfile.getGpuName());
        LOGGER.info("  RAM: {} MB", hardwareProfile.getTotalMemoryMB());
        LOGGER.info("  Hardware Hash: {}", hardwareProfile.getHardwareHash());

        // Step 3: Check Vulkan support
        VulkanCapabilities vulkanCaps = null;
        if (config.isVulkanEnabled()) {
            LOGGER.info("[3/7] Checking Vulkan capabilities...");
            try {
                vulkanCaps = VulkanCapabilities.detect();
                LOGGER.info("  Vulkan Version: {}", vulkanCaps.getVersionString());
                LOGGER.info("  Max Image Dimension: {}", vulkanCaps.getMaxImageDimension2D());
                LOGGER.info("  Compute Shaders: {}", vulkanCaps.isComputeSupported() ? "Yes" : "No");
                LOGGER.info("  Ray Tracing: {}", vulkanCaps.isRayTracingSupported() ? "Yes" : "No");
            } catch (Exception e) {
                LOGGER.warn("Vulkan not available: {}. Falling back to OpenGL.", e.getMessage());
            }
        } else {
            LOGGER.info("[3/7] Vulkan disabled by configuration");
        }

        // Step 4: Extract native libraries
        initState.set(InitState.EXTRACTING_NATIVES);
        LOGGER.info("[4/7] Extracting native libraries...");
        NativeLoader nativeLoader = new NativeLoader(hardwareProfile);
        NativeExtractionResult extractResult = nativeLoader.extractNatives();
        LOGGER.info("  Extracted to: {}", extractResult.getPath());
        LOGGER.info("  Size: {} KB", extractResult.getSizeBytes() / 1024);
        LOGGER.info("  Hash verified: {}", extractResult.isHashValid() ? "Yes" : "No");

        // Step 5: Load native libraries
        initState.set(InitState.LOADING_NATIVES);
        LOGGER.info("[5/7] Loading native libraries...");
        int retries = 0;
        Exception lastException = null;
        while (retries < MAX_NATIVE_LOAD_RETRIES) {
            try {
                // Use NativeBridge to load the library so it knows it's loaded
                // and can fetch primitive version info
                NativeBridge.loadLibrary(extractResult.getPath());
                LOGGER.info("  Native library loaded successfully");
                break;
            } catch (UnsatisfiedLinkError | Exception e) {
                lastException = e instanceof Exception ? (Exception) e : new Exception(e);
                retries++;
                LOGGER.warn("  Load attempt {} failed: {}", retries, e.getMessage());
                if (retries < MAX_NATIVE_LOAD_RETRIES) {
                    Thread.sleep(100 * retries);
                }
            }
        }
        if (retries >= MAX_NATIVE_LOAD_RETRIES) {
            throw new LibsInitializationException("Failed to load native library after " +
                    MAX_NATIVE_LOAD_RETRIES + " attempts", lastException);
        }

        // Step 6: Create core instance and initialize engine
        initState.set(InitState.INITIALIZING_ENGINE);
        LOGGER.info("[6/7] Initializing Libs engine...");
        LibsCore core = new LibsCore(config, hardwareProfile, vulkanCaps, extractResult.getPath());

        // Initialize native engine
        boolean engineInit = core.nativeBridge.initializeEngine(
                hardwareProfile.toNativeFormat(),
                vulkanCaps != null ? vulkanCaps.toNativeFormat() : null,
                config.toNativeFormat());

        if (!engineInit) {
            throw new LibsInitializationException("Native engine initialization failed");
        }

        // Step 7: Start subsystems
        initState.set(InitState.STARTING_SUBSYSTEMS);
        LOGGER.info("[7/7] Starting subsystems...");
        core.startSubsystems();

        // Mark as running
        initState.set(InitState.RUNNING);
        core.running.set(true);

        long elapsed = System.currentTimeMillis() - startTime;
        LOGGER.info("╔══════════════════════════════════════════════════════════════════╗");
        LOGGER.info("║              LIBS initialization COMPLETE                       ║");
        LOGGER.info("║              Time: {} ms | Mode: {}                               ║",
                elapsed, core.operationMode);
        LOGGER.info("╚══════════════════════════════════════════════════════════════════╝");

        return core;
    }

    // =========================================================================
    // CONFIGURATION
    // =========================================================================

    /**
     * Load configuration from file or create default
     */
    private static LibsConfig loadConfiguration(@Nullable Path configPath) throws IOException {
        if (configPath == null) {
            // Look for config in standard locations
            Path[] searchPaths = {
                    Paths.get("config", "Libs.json"),
                    Paths.get("Libs.json"),
                    Paths.get(System.getProperty("user.home"), ".Libs", "config.json")
            };

            for (Path path : searchPaths) {
                if (Files.exists(path)) {
                    configPath = path;
                    break;
                }
            }
        }

        if (configPath != null && Files.exists(configPath)) {
            LOGGER.info("  Loading config from: {}", configPath);
            return LibsConfig.load(configPath);
        } else {
            LOGGER.info("  Using default configuration");
            return LibsConfig.createDefault();
        }
    }

    // =========================================================================
    // OPERATION MODE
    // =========================================================================

    /**
     * Determine the best operation mode based on hardware and configuration
     */
    private OperationMode determineOperationMode() {
        // Check if forced to safe mode
        if (config.isForceSafeMode()) {
            return OperationMode.SAFE_MODE;
        }

        // Check if disabled
        if (!config.isEnabled()) {
            return OperationMode.DISABLED;
        }

        // Check Vulkan availability
        if (vulkanCaps == null) {
            LOGGER.info("Vulkan not available - using OpenGL fallback");
            return OperationMode.OPENGL_FALLBACK;
        }

        // Check if hybrid mode is preferred
        if (config.isHybridModePreferred()) {
            return OperationMode.VULKAN_HYBRID;
        }

        // Full Vulkan if everything is supported
        if (vulkanCaps.isFullySupported()) {
            return OperationMode.VULKAN_FULL;
        }

        // Default to hybrid
        return OperationMode.VULKAN_HYBRID;
    }

    // =========================================================================
    // SUBSYSTEM MANAGEMENT
    // =========================================================================

    /**
     * Start all subsystems
     */
    private void startSubsystems() {
        // Start memory manager
        startSubsystem("memory", () -> {
            voidManager.initialize();
            return true;
        });

        // Start asset interceptor
        startSubsystem("assets", () -> {
            assetInterceptor.initialize();
            return true;
        });

        // Start render hijacker (if not disabled)
        if (operationMode != OperationMode.DISABLED) {
            startSubsystem("render", () -> {
                renderHijacker.initialize();
                return true;
            });
        }

        // Start chunk extractor
        startSubsystem("chunks", () -> {
            chunkExtractor.initialize();
            return true;
        });

        // Start network module
        startSubsystem("network", () -> {
            netcode.initialize();
            return true;
        });

        // Schedule periodic tasks
        schedulePeriodicTasks();

        LOGGER.info("  All subsystems started");
    }

    /**
     * Pre-initialize the engine
     */
    public void preInit() {
        // Start initialization in background if not already started
        if (initState.get() == InitState.NOT_STARTED) {
            initialize(null);
        }
    }

    /**
     * Initialize the engine
     */
    public void init() {
        if (initState.get() == InitState.NOT_STARTED) {
            initialize(null);
        }
    }

    /**
     * Server tick
     */
    public void serverTick() {
        if (!running.get())
            return;

        // Update network
        if (netcode != null) {
            netcode.update(0.05f); // 20 TPS = 0.05s
        }

        // Native tick
        if (nativeBridge != null) {
            nativeBridge.serverTick();
        }
    }

    /**
     * Client tick
     */
    public void clientTick() {
        if (!running.get())
            return;

        // Native tick
        if (nativeBridge != null) {
            nativeBridge.clientTick();
        }
    }

    /**
     * Start a single subsystem with error handling
     */
    private void startSubsystem(String name, Supplier<Boolean> initializer) {
        subsystemStatus.put(name, SubsystemStatus.STARTING);
        try {
            if (initializer.get()) {
                subsystemStatus.put(name, SubsystemStatus.RUNNING);
                LOGGER.info("  Subsystem '{}' started", name);
            } else {
                subsystemStatus.put(name, SubsystemStatus.DEGRADED);
                LOGGER.warn("  Subsystem '{}' started in degraded mode", name);
            }
        } catch (Exception e) {
            subsystemStatus.put(name, SubsystemStatus.FAILED);
            LOGGER.error("  Subsystem '{}' failed to start: {}", name, e.getMessage());

            // Notify listeners
            fireEvent(new LibsEvent.SubsystemFailedEvent(name, e));
        }
    }

    /**
     * Schedule periodic maintenance tasks
     */
    private void schedulePeriodicTasks() {
        // Native memory GC sync
        scheduledExecutor.scheduleAtFixedRate(
                this::performNativeGC,
                NATIVE_GC_INTERVAL_MS,
                NATIVE_GC_INTERVAL_MS,
                TimeUnit.MILLISECONDS);

        // Statistics update
        scheduledExecutor.scheduleAtFixedRate(
                this::updateStatistics,
                1000,
                1000,
                TimeUnit.MILLISECONDS);

        // Health check
        scheduledExecutor.scheduleAtFixedRate(
                this::performHealthCheck,
                5000,
                5000,
                TimeUnit.MILLISECONDS);
    }

    /**
     * Perform native garbage collection sync
     */
    private void performNativeGC() {
        if (!running.get() || paused.get())
            return;

        try {
            voidManager.collectGarbage();
            nativeBridge.syncMemory();
        } catch (Exception e) {
            LOGGER.debug("Native GC sync failed: {}", e.getMessage());
        }
    }

    /**
     * Update statistics
     */
    private void updateStatistics() {
        if (!running.get())
            return;

        statistics.update(
                frameCounter.get(),
                tickCounter.get(),
                voidManager.getAllocatedBytes(),
                nativeBridge.getNativeMemoryUsage());
    }

    /**
     * Perform health check on all subsystems
     */
    private void performHealthCheck() {
        if (!running.get())
            return;

        for (Map.Entry<String, SubsystemStatus> entry : subsystemStatus.entrySet()) {
            if (entry.getValue() == SubsystemStatus.RUNNING) {
                // Check if subsystem is still healthy
                boolean healthy = checkSubsystemHealth(entry.getKey());
                if (!healthy) {
                    subsystemStatus.put(entry.getKey(), SubsystemStatus.DEGRADED);
                    fireEvent(new LibsEvent.SubsystemDegradedEvent(entry.getKey()));
                }
            }
        }
    }

    /**
     * Check health of a specific subsystem
     */
    private boolean checkSubsystemHealth(String name) {
        try {
            switch (name) {
                case "memory":
                    return voidManager.isHealthy();
                case "render":
                    return renderHijacker.isHealthy();
                case "network":
                    return netcode.isHealthy();
                default:
                    return true;
            }
        } catch (Exception e) {
            return false;
        }
    }

    // =========================================================================
    // RUNTIME METHODS
    // =========================================================================

    /**
     * Called every game tick
     * 
     * @param deltaTime Time since last tick in seconds
     */
    public void onTick(float deltaTime) {
        if (!running.get() || paused.get())
            return;

        long tick = tickCounter.incrementAndGet();

        // Send tick to native engine
        nativeBridge.onTick(tick, deltaTime);

        // Update network prediction
        netcode.update(deltaTime);

        // Fire tick event
        fireEvent(new LibsEvent.TickEvent(tick, deltaTime));
    }

    /**
     * Called before rendering a frame
     * 
     * @param partialTicks Partial tick time for interpolation
     */
    public void onPreRender(float partialTicks) {
        if (!running.get() || paused.get())
            return;

        // Prepare render data
        renderHijacker.prepareFrame(partialTicks);
    }

    /**
     * Called to render the world
     * 
     * @param partialTicks Partial tick time
     * @param viewMatrix   Camera view matrix (16 floats)
     * @param projMatrix   Camera projection matrix (16 floats)
     */
    public void onRenderWorld(float partialTicks, float[] viewMatrix, float[] projMatrix) {
        if (!running.get() || paused.get())
            return;
        if (operationMode == OperationMode.DISABLED)
            return;

        long frame = frameCounter.incrementAndGet();

        // Delegate to native bridge for rendering
        nativeBridge.renderWorld(frame, partialTicks, viewMatrix, projMatrix);
    }

    /**
     * Called after the UI has been rendered (for composition)
     */
    public void onPostRender() {
        if (!running.get() || paused.get())
            return;
        if (operationMode == OperationMode.DISABLED)
            return;

        // Composite Vulkan frame with OpenGL UI
        renderHijacker.compositeFrame();
    }

    /**
     * Notify of a chunk update
     * 
     * @param chunkX Chunk X coordinate
     * @param chunkZ Chunk Z coordinate
     */
    public void onChunkUpdate(int chunkX, int chunkZ) {
        if (!running.get())
            return;

        // Queue chunk for re-meshing
        chunkExtractor.markChunkDirty(chunkX, chunkZ);
    }

    /**
     * Notify of entity spawn
     * 
     * @param entityId   Entity ID
     * @param entityType Entity type identifier
     * @param x          X position
     * @param y          Y position
     * @param z          Z position
     */
    public void onEntitySpawn(int entityId, String entityType, double x, double y, double z) {
        if (!running.get())
            return;

        // Register entity in native ECS
        nativeBridge.spawnEntity(entityId, entityType, x, y, z);
    }

    /**
     * Notify of entity removal
     * 
     * @param entityId Entity ID to remove
     */
    public void onEntityRemove(int entityId) {
        if (!running.get())
            return;

        nativeBridge.removeEntity(entityId);
    }

    // =========================================================================
    // EVENT SYSTEM
    // =========================================================================

    /**
     * Register an event listener
     * 
     * @param listener Listener to register
     */
    public void addEventListener(LibsEventListener listener) {
        eventListeners.add(listener);
    }

    /**
     * Remove an event listener
     * 
     * @param listener Listener to remove
     */
    public void removeEventListener(LibsEventListener listener) {
        eventListeners.remove(listener);
    }

    /**
     * Fire an event to all listeners
     */
    private void fireEvent(LibsEvent event) {
        for (LibsEventListener listener : eventListeners) {
            try {
                listener.onEvent(event);
            } catch (Exception e) {
                LOGGER.error("Event listener error: {}", e.getMessage());
            }
        }
    }

    // =========================================================================
    // LIFECYCLE METHODS
    // =========================================================================

    /**
     * Pause the engine
     */
    public void pause() {
        if (paused.compareAndSet(false, true)) {
            LOGGER.info("Libs engine paused");
            nativeBridge.pause();
            fireEvent(new LibsEvent.PauseEvent());
        }
    }

    /**
     * Resume the engine
     */
    public void resume() {
        if (paused.compareAndSet(true, false)) {
            LOGGER.info("Libs engine resumed");
            nativeBridge.resume();
            fireEvent(new LibsEvent.ResumeEvent());
        }
    }

    /**
     * Shutdown the engine
     */
    public void shutdown() {
        if (!running.compareAndSet(true, false)) {
            return; // Already shut down or shutting down
        }

        initState.set(InitState.SHUTTING_DOWN);
        LOGGER.info("Shutting down Libs engine...");

        try {
            // Fire shutdown event
            fireEvent(new LibsEvent.ShutdownEvent());

            // Stop scheduled tasks
            scheduledExecutor.shutdown();

            // Shutdown subsystems in reverse order
            LOGGER.info("  Stopping network...");
            netcode.shutdown();

            LOGGER.info("  Stopping render...");
            renderHijacker.shutdown();

            LOGGER.info("  Stopping assets...");
            assetInterceptor.shutdown();

            LOGGER.info("  Stopping memory manager...");
            voidManager.shutdown();

            // Shutdown native engine
            LOGGER.info("  Shutting down native engine...");
            nativeBridge.shutdown();

            // Shutdown executors
            mainExecutor.shutdown();
            if (!mainExecutor.awaitTermination(5, TimeUnit.SECONDS)) {
                mainExecutor.shutdownNow();
            }

            if (!scheduledExecutor.awaitTermination(5, TimeUnit.SECONDS)) {
                scheduledExecutor.shutdownNow();
            }

            initState.set(InitState.SHUTDOWN);
            LOGGER.info("Libs engine shutdown complete");

        } catch (Exception e) {
            LOGGER.error("Error during shutdown: {}", e.getMessage());
        }
    }

    // =========================================================================
    // GETTERS
    // =========================================================================

    /**
     * Get the singleton instance
     * 
     * @return The LibsCore instance, or null if not initialized
     */
    @Nullable
    public static LibsCore getInstance() {
        return instance;
    }

    /**
     * Get the singleton instance, throwing if not initialized
     * 
     * @return The LibsCore instance
     * @throws IllegalStateException if not initialized
     */
    @NotNull
    public static LibsCore getInstanceOrThrow() {
        LibsCore core = instance;
        if (core == null) {
            throw new IllegalStateException("LibsCore not initialized. Call initialize() first.");
        }
        return core;
    }

    /**
     * Check if the engine is initialized
     */
    public static boolean isInitialized() {
        return instance != null && initState.get() == InitState.RUNNING;
    }

    /**
     * Get the current initialization state
     */
    public static InitState getInitState() {
        return initState.get();
    }

    public LibsConfig getConfig() {
        return config;
    }

    public HardwareProfile getHardwareProfile() {
        return hardwareProfile;
    }

    public VulkanCapabilities getVulkanCapabilities() {
        return vulkanCaps;
    }

    public OperationMode getOperationMode() {
        return operationMode;
    }

    public NativeBridge getNativeBridge() {
        return nativeBridge;
    }

    public VoidManager getVoidManager() {
        return voidManager;
    }

    public RenderHijacker getRenderHijacker() {
        return renderHijacker;
    }

    public PredictiveNetcode getNetcode() {
        return netcode;
    }

    public LibsStatistics getStatistics() {
        return statistics;
    }

    public long getFrameCount() {
        return frameCounter.get();
    }

    public long getTickCount() {
        return tickCounter.get();
    }

    public boolean isRunning() {
        return running.get();
    }

    public boolean isPaused() {
        return paused.get();
    }

    public long getUptimeMs() {
        return System.currentTimeMillis() - startupTimestamp;
    }

    /**
     * Get subsystem status
     */
    public SubsystemStatus getSubsystemStatus(String name) {
        return subsystemStatus.getOrDefault(name, SubsystemStatus.UNINITIALIZED);
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Statistics tracking class
     */
    public static class LibsStatistics {
        private volatile long frames;
        private volatile long ticks;
        private volatile long javaMemoryBytes;
        private volatile long nativeMemoryBytes;
        private volatile double fps;
        private volatile double tps;
        private long lastUpdate;
        private long lastFrames;
        private long lastTicks;

        void update(long frames, long ticks, long javaMemory, long nativeMemory) {
            long now = System.currentTimeMillis();
            long elapsed = now - lastUpdate;

            if (elapsed > 0) {
                this.fps = (frames - lastFrames) * 1000.0 / elapsed;
                this.tps = (ticks - lastTicks) * 1000.0 / elapsed;
            }

            this.frames = frames;
            this.ticks = ticks;
            this.javaMemoryBytes = javaMemory;
            this.nativeMemoryBytes = nativeMemory;
            this.lastUpdate = now;
            this.lastFrames = frames;
            this.lastTicks = ticks;
        }

        public long getFrames() {
            return frames;
        }

        public long getTicks() {
            return ticks;
        }

        public long getJavaMemoryBytes() {
            return javaMemoryBytes;
        }

        public long getNativeMemoryBytes() {
            return nativeMemoryBytes;
        }

        public long getTotalMemoryBytes() {
            return javaMemoryBytes + nativeMemoryBytes;
        }

        public double getFps() {
            return fps;
        }

        public double getTps() {
            return tps;
        }
    }

    /**
     * Event listener interface
     */
    public interface LibsEventListener {
        void onEvent(LibsEvent event);
    }

    /**
     * Base event class
     */
    public static abstract class LibsEvent {
        private final long timestamp = System.currentTimeMillis();

        public long getTimestamp() {
            return timestamp;
        }

        public static class TickEvent extends LibsEvent {
            public final long tick;
            public final float deltaTime;

            TickEvent(long tick, float deltaTime) {
                this.tick = tick;
                this.deltaTime = deltaTime;
            }
        }

        public static class PauseEvent extends LibsEvent {
        }

        public static class ResumeEvent extends LibsEvent {
        }

        public static class ShutdownEvent extends LibsEvent {
        }

        public static class SubsystemFailedEvent extends LibsEvent {
            public final String subsystem;
            public final Exception error;

            SubsystemFailedEvent(String subsystem, Exception error) {
                this.subsystem = subsystem;
                this.error = error;
            }
        }

        public static class SubsystemDegradedEvent extends LibsEvent {
            public final String subsystem;

            SubsystemDegradedEvent(String subsystem) {
                this.subsystem = subsystem;
            }
        }
    }

    /**
     * Custom exception for initialization failures
     */
    public static class LibsInitializationException extends RuntimeException {
        public LibsInitializationException(String message) {
            super(message);
        }

        public LibsInitializationException(String message, Throwable cause) {
            super(message, cause);
        }
    }
}
