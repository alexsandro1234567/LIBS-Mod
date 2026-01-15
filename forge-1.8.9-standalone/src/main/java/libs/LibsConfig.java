/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * LibsConfig.java - Configuration Management System
 * 
 * Handles all configuration for the Libs engine including:
 * - Render settings (Vulkan/OpenGL modes)
 * - Memory management settings
 * - Network settings
 * - Performance profiles
 * - Compatibility options
 */

package dev.libs;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Consumer;

/**
 * LibsConfig - Comprehensive Configuration System
 * 
 * This class manages all configurable aspects of the Libs engine.
 * Configuration is persisted in JSON format and supports hot-reloading.
 * 
 * <h2>Configuration Structure:</h2>
 * 
 * <pre>
 * {
 *   "general": { ... },
 *   "render": { ... },
 *   "memory": { ... },
 *   "network": { ... },
 *   "physics": { ... },
 *   "audio": { ... },
 *   "compatibility": { ... },
 *   "debug": { ... }
 * }
 * </pre>
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class LibsConfig {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(LibsConfig.class);

    /** Current configuration schema version */
    public static final int SCHEMA_VERSION = 1;

    /** Default configuration file name */
    public static final String DEFAULT_FILE_NAME = "Libs.json";

    private static final Gson GSON = new GsonBuilder()
            .setPrettyPrinting()
            .excludeFieldsWithoutExposeAnnotation()
            .serializeNulls()
            .create();

    // =========================================================================
    // CONFIGURATION SECTIONS
    // =========================================================================

    @Expose
    @SerializedName("schema_version")
    private int schemaVersion = SCHEMA_VERSION;

    @Expose
    @SerializedName("general")
    private GeneralConfig general = new GeneralConfig();

    @Expose
    @SerializedName("render")
    private RenderConfig render = new RenderConfig();

    @Expose
    @SerializedName("memory")
    private MemoryConfig memory = new MemoryConfig();

    @Expose
    @SerializedName("network")
    private NetworkConfig network = new NetworkConfig();

    @Expose
    @SerializedName("physics")
    private PhysicsConfig physics = new PhysicsConfig();

    @Expose
    @SerializedName("audio")
    private AudioConfig audio = new AudioConfig();

    @Expose
    @SerializedName("compatibility")
    private CompatibilityConfig compatibility = new CompatibilityConfig();

    @Expose
    @SerializedName("debug")
    private DebugConfig debug = new DebugConfig();

    // =========================================================================
    // RUNTIME STATE
    // =========================================================================

    /** Path to the configuration file */
    private transient Path configPath;

    /** Change listeners */
    private transient final List<Consumer<LibsConfig>> changeListeners = new ArrayList<>();

    /** Last modification time of the config file */
    private transient long lastModifiedTime = 0;

    // =========================================================================
    // SECTION GETTERS
    // =========================================================================

    /** Get general configuration */
    public GeneralConfig getGeneralConfig() {
        return general;
    }

    /** Get render configuration */
    public RenderConfig getRenderConfig() {
        return render;
    }

    /** Get memory configuration */
    public MemoryConfig getMemoryConfig() {
        return memory;
    }

    /** Get network configuration */
    public NetworkConfig getNetworkConfig() {
        return network;
    }

    /** Get physics configuration */
    public PhysicsConfig getPhysicsConfig() {
        return physics;
    }

    /** Get audio configuration */
    public AudioConfig getAudioConfig() {
        return audio;
    }

    /** Get compatibility configuration */
    public CompatibilityConfig getCompatibilityConfig() {
        return compatibility;
    }

    /** Get debug configuration */
    public DebugConfig getDebugConfig() {
        return debug;
    }

    // =========================================================================
    // GENERAL CONFIGURATION
    // =========================================================================

    /**
     * General engine settings
     */
    public static class GeneralConfig {

        @Expose
        @SerializedName("enabled")
        private boolean enabled = true;

        @Expose
        @SerializedName("force_safe_mode")
        private boolean forceSafeMode = false;

        @Expose
        @SerializedName("auto_detect_hardware")
        private boolean autoDetectHardware = true;

        @Expose
        @SerializedName("performance_profile")
        private PerformanceProfile performanceProfile = PerformanceProfile.BALANCED;

        @Expose
        @SerializedName("locale")
        private String locale = "en_US";

        @Expose
        @SerializedName("check_updates")
        private boolean checkUpdates = true;

        @Expose
        @SerializedName("telemetry_enabled")
        private boolean telemetryEnabled = false;

        @Expose
        @SerializedName("native_extraction_path")
        private String nativeExtractionPath = null; // null = auto (/tmp or %TEMP%)

        @Expose
        @SerializedName("show_extraction_logs")
        private boolean showExtractionLogs = true;

        @Expose
        @SerializedName("verify_binary_hashes")
        private boolean verifyBinaryHashes = true;

        // Getters and setters
        public boolean isEnabled() {
            return enabled;
        }

        public void setEnabled(boolean enabled) {
            this.enabled = enabled;
        }

        public boolean isForceSafeMode() {
            return forceSafeMode;
        }

        public void setForceSafeMode(boolean forceSafeMode) {
            this.forceSafeMode = forceSafeMode;
        }

        public boolean isAutoDetectHardware() {
            return autoDetectHardware;
        }

        public void setAutoDetectHardware(boolean autoDetectHardware) {
            this.autoDetectHardware = autoDetectHardware;
        }

        public PerformanceProfile getPerformanceProfile() {
            return performanceProfile;
        }

        public void setPerformanceProfile(PerformanceProfile profile) {
            this.performanceProfile = profile;
        }

        public String getLocale() {
            return locale;
        }

        public void setLocale(String locale) {
            this.locale = locale;
        }

        public boolean isCheckUpdates() {
            return checkUpdates;
        }

        public void setCheckUpdates(boolean checkUpdates) {
            this.checkUpdates = checkUpdates;
        }

        public boolean isTelemetryEnabled() {
            return telemetryEnabled;
        }

        public void setTelemetryEnabled(boolean enabled) {
            this.telemetryEnabled = enabled;
        }

        public String getNativeExtractionPath() {
            return nativeExtractionPath;
        }

        public void setNativeExtractionPath(String path) {
            this.nativeExtractionPath = path;
        }

        public boolean isShowExtractionLogs() {
            return showExtractionLogs;
        }

        public void setShowExtractionLogs(boolean show) {
            this.showExtractionLogs = show;
        }

        public boolean isVerifyBinaryHashes() {
            return verifyBinaryHashes;
        }

        public void setVerifyBinaryHashes(boolean verify) {
            this.verifyBinaryHashes = verify;
        }
    }

    /**
     * Performance profiles
     */
    public enum PerformanceProfile {
        /** Minimal resource usage, lower quality */
        LOW,
        /** Balanced performance and quality */
        BALANCED,
        /** High quality, more resource usage */
        HIGH,
        /** Maximum quality, enthusiast hardware */
        ULTRA,
        /** Custom settings */
        CUSTOM
    }

    // =========================================================================
    // RENDER CONFIGURATION
    // =========================================================================

    /**
     * Rendering engine settings
     */
    public static class RenderConfig {

        @Expose
        @SerializedName("vulkan_enabled")
        private boolean vulkanEnabled = true;

        @Expose
        @SerializedName("hybrid_mode")
        private boolean hybridMode = true;

        @Expose
        @SerializedName("render_distance")
        private int renderDistance = 16;

        @Expose
        @SerializedName("max_render_distance")
        private int maxRenderDistance = 64;

        @Expose
        @SerializedName("far_render_distance")
        private int farRenderDistance = 256; // SDF ray-marched

        @Expose
        @SerializedName("target_fps")
        private int targetFps = 60;

        @Expose
        @SerializedName("vsync")
        private boolean vsync = true;

        @Expose
        @SerializedName("vsync_mode")
        private VSyncMode vsyncMode = VSyncMode.ADAPTIVE;

        @Expose
        @SerializedName("resolution_scale")
        private float resolutionScale = 1.0f;

        @Expose
        @SerializedName("nanite_enabled")
        private boolean naniteEnabled = true;

        @Expose
        @SerializedName("nanite_lod_bias")
        private float naniteLodBias = 0.0f;

        @Expose
        @SerializedName("sdf_enabled")
        private boolean sdfEnabled = true;

        @Expose
        @SerializedName("greedy_meshing")
        private boolean greedyMeshing = true;

        @Expose
        @SerializedName("frustum_culling")
        private boolean frustumCulling = true;

        @Expose
        @SerializedName("occlusion_culling")
        private boolean occlusionCulling = true;

        @Expose
        @SerializedName("shadow_quality")
        private QualityLevel shadowQuality = QualityLevel.HIGH;

        @Expose
        @SerializedName("shadow_distance")
        private int shadowDistance = 64;

        @Expose
        @SerializedName("ssgi_enabled")
        private boolean ssgiEnabled = true;

        @Expose
        @SerializedName("ssgi_quality")
        private QualityLevel ssgiQuality = QualityLevel.MEDIUM;

        @Expose
        @SerializedName("ao_enabled")
        private boolean aoEnabled = true;

        @Expose
        @SerializedName("ao_type")
        private AOType aoType = AOType.GTAO;

        @Expose
        @SerializedName("bloom_enabled")
        private boolean bloomEnabled = true;

        @Expose
        @SerializedName("motion_blur_enabled")
        private boolean motionBlurEnabled = false;

        @Expose
        @SerializedName("chromatic_aberration")
        private boolean chromaticAberration = false;

        @Expose
        @SerializedName("anti_aliasing")
        private AntiAliasingMode antiAliasing = AntiAliasingMode.TAA;

        @Expose
        @SerializedName("texture_quality")
        private QualityLevel textureQuality = QualityLevel.HIGH;

        @Expose
        @SerializedName("anisotropic_filtering")
        private int anisotropicFiltering = 16;

        @Expose
        @SerializedName("bindless_textures")
        private boolean bindlessTextures = true;

        @Expose
        @SerializedName("ray_tracing_enabled")
        private boolean rayTracingEnabled = false;

        @Expose
        @SerializedName("rt_reflections")
        private boolean rtReflections = false;

        @Expose
        @SerializedName("rt_shadows")
        private boolean rtShadows = false;

        @Expose
        @SerializedName("rt_gi")
        private boolean rtGlobalIllumination = false;

        // Getters
        public boolean isVulkanEnabled() {
            return vulkanEnabled;
        }

        public boolean isHybridMode() {
            return hybridMode;
        }

        public int getRenderDistance() {
            return renderDistance;
        }

        public int getMaxRenderDistance() {
            return maxRenderDistance;
        }

        public int getFarRenderDistance() {
            return farRenderDistance;
        }

        public int getTargetFps() {
            return targetFps;
        }

        public boolean isVsync() {
            return vsync;
        }

        public VSyncMode getVsyncMode() {
            return vsyncMode;
        }

        public float getResolutionScale() {
            return resolutionScale;
        }

        public boolean isNaniteEnabled() {
            return naniteEnabled;
        }

        public float getNaniteLodBias() {
            return naniteLodBias;
        }

        public boolean isSdfEnabled() {
            return sdfEnabled;
        }

        public boolean isGreedyMeshing() {
            return greedyMeshing;
        }

        public boolean isFrustumCulling() {
            return frustumCulling;
        }

        public boolean isOcclusionCulling() {
            return occlusionCulling;
        }

        public QualityLevel getShadowQuality() {
            return shadowQuality;
        }

        public int getShadowDistance() {
            return shadowDistance;
        }

        public boolean isSsgiEnabled() {
            return ssgiEnabled;
        }

        public QualityLevel getSsgiQuality() {
            return ssgiQuality;
        }

        public boolean isAoEnabled() {
            return aoEnabled;
        }

        public AOType getAoType() {
            return aoType;
        }

        public boolean isBloomEnabled() {
            return bloomEnabled;
        }

        public boolean isMotionBlurEnabled() {
            return motionBlurEnabled;
        }

        public boolean isChromaticAberration() {
            return chromaticAberration;
        }

        public AntiAliasingMode getAntiAliasing() {
            return antiAliasing;
        }

        public QualityLevel getTextureQuality() {
            return textureQuality;
        }

        public int getAnisotropicFiltering() {
            return anisotropicFiltering;
        }

        public boolean isBindlessTextures() {
            return bindlessTextures;
        }

        public boolean isRayTracingEnabled() {
            return rayTracingEnabled;
        }

        public boolean isRtReflections() {
            return rtReflections;
        }

        public boolean isRtShadows() {
            return rtShadows;
        }

        public boolean isRtGlobalIllumination() {
            return rtGlobalIllumination;
        }

        // Setters
        public void setVulkanEnabled(boolean v) {
            this.vulkanEnabled = v;
        }

        public void setHybridMode(boolean v) {
            this.hybridMode = v;
        }

        public void setRenderDistance(int v) {
            this.renderDistance = Math.max(2, Math.min(v, maxRenderDistance));
        }

        public void setMaxRenderDistance(int v) {
            this.maxRenderDistance = v;
        }

        public void setFarRenderDistance(int v) {
            this.farRenderDistance = v;
        }

        public void setTargetFps(int v) {
            this.targetFps = Math.max(30, v);
        }

        public void setVsync(boolean v) {
            this.vsync = v;
        }

        public void setVsyncMode(VSyncMode v) {
            this.vsyncMode = v;
        }

        public void setResolutionScale(float v) {
            this.resolutionScale = Math.max(0.25f, Math.min(v, 2.0f));
        }

        public void setNaniteEnabled(boolean v) {
            this.naniteEnabled = v;
        }

        public void setNaniteLodBias(float v) {
            this.naniteLodBias = v;
        }

        public void setSdfEnabled(boolean v) {
            this.sdfEnabled = v;
        }

        public void setGreedyMeshing(boolean v) {
            this.greedyMeshing = v;
        }

        public void setFrustumCulling(boolean v) {
            this.frustumCulling = v;
        }

        public void setOcclusionCulling(boolean v) {
            this.occlusionCulling = v;
        }

        public void setShadowQuality(QualityLevel v) {
            this.shadowQuality = v;
        }

        public void setShadowDistance(int v) {
            this.shadowDistance = v;
        }

        public void setSsgiEnabled(boolean v) {
            this.ssgiEnabled = v;
        }

        public void setSsgiQuality(QualityLevel v) {
            this.ssgiQuality = v;
        }

        public void setAoEnabled(boolean v) {
            this.aoEnabled = v;
        }

        public void setAoType(AOType v) {
            this.aoType = v;
        }

        public void setBloomEnabled(boolean v) {
            this.bloomEnabled = v;
        }

        public void setMotionBlurEnabled(boolean v) {
            this.motionBlurEnabled = v;
        }

        public void setChromaticAberration(boolean v) {
            this.chromaticAberration = v;
        }

        public void setAntiAliasing(AntiAliasingMode v) {
            this.antiAliasing = v;
        }

        public void setTextureQuality(QualityLevel v) {
            this.textureQuality = v;
        }

        public void setAnisotropicFiltering(int v) {
            this.anisotropicFiltering = Math.max(1, Math.min(v, 16));
        }

        public void setBindlessTextures(boolean v) {
            this.bindlessTextures = v;
        }

        public void setRayTracingEnabled(boolean v) {
            this.rayTracingEnabled = v;
        }

        public void setRtReflections(boolean v) {
            this.rtReflections = v;
        }

        public void setRtShadows(boolean v) {
            this.rtShadows = v;
        }

        public void setRtGlobalIllumination(boolean v) {
            this.rtGlobalIllumination = v;
        }

        // Convenience methods for compatibility
        public RenderMode getMode() {
            if (vulkanEnabled && hybridMode) {
                return RenderMode.HYBRID;
            } else if (vulkanEnabled) {
                return RenderMode.VULKAN;
            } else {
                return RenderMode.OPENGL;
            }
        }

        public int getMaxFps() {
            return targetFps;
        }

        public float getRenderScale() {
            return resolutionScale;
        }
    }

    public enum VSyncMode {
        OFF, ON, ADAPTIVE, TRIPLE_BUFFERED
    }

    public enum QualityLevel {
        OFF, LOW, MEDIUM, HIGH, ULTRA
    }

    public enum AOType {
        OFF, SSAO, HBAO, GTAO
    }

    public enum AntiAliasingMode {
        OFF, FXAA, SMAA, TAA, DLSS, FSR
    }

    public enum RenderMode {
        VULKAN, OPENGL, HYBRID
    }

    // Helper methods for RenderConfig compatibility
    public static class RenderConfigHelper {
        public static RenderMode getMode(RenderConfig config) {
            if (config.isVulkanEnabled() && config.isHybridMode()) {
                return RenderMode.HYBRID;
            } else if (config.isVulkanEnabled()) {
                return RenderMode.VULKAN;
            } else {
                return RenderMode.OPENGL;
            }
        }

        public static int getMaxFps(RenderConfig config) {
            return config.getTargetFps();
        }

        public static float getRenderScale(RenderConfig config) {
            return config.getResolutionScale();
        }
    }

    // =========================================================================
    // MEMORY CONFIGURATION
    // =========================================================================

    /**
     * Memory management settings
     */
    public static class MemoryConfig {

        @Expose
        @SerializedName("off_heap_enabled")
        private boolean offHeapEnabled = true;

        @Expose
        @SerializedName("off_heap_max_mb")
        private int offHeapMaxMB = 4096;

        @Expose
        @SerializedName("texture_streaming")
        private boolean textureStreaming = true;

        @Expose
        @SerializedName("texture_budget_mb")
        private int textureBudgetMB = 2048;

        @Expose
        @SerializedName("mesh_budget_mb")
        private int meshBudgetMB = 1024;

        @Expose
        @SerializedName("deduplication_enabled")
        private boolean deduplicationEnabled = true;

        @Expose
        @SerializedName("gc_sync_interval_ms")
        private int gcSyncIntervalMs = 5000;

        @Expose
        @SerializedName("preload_chunks")
        private boolean preloadChunks = true;

        @Expose
        @SerializedName("preload_distance")
        private int preloadDistance = 4;

        @Expose
        @SerializedName("aggressive_unloading")
        private boolean aggressiveUnloading = false;

        @Expose
        @SerializedName("arena_allocator")
        private boolean arenaAllocator = true;

        @Expose
        @SerializedName("pool_small_objects")
        private boolean poolSmallObjects = true;

        // Getters
        public boolean isOffHeapEnabled() {
            return offHeapEnabled;
        }

        public int getOffHeapMaxMB() {
            return offHeapMaxMB;
        }

        public boolean isTextureStreaming() {
            return textureStreaming;
        }

        public int getTextureBudgetMB() {
            return textureBudgetMB;
        }

        public int getMeshBudgetMB() {
            return meshBudgetMB;
        }

        public boolean isDeduplicationEnabled() {
            return deduplicationEnabled;
        }

        public int getGcSyncIntervalMs() {
            return gcSyncIntervalMs;
        }

        public boolean isPreloadChunks() {
            return preloadChunks;
        }

        public int getPreloadDistance() {
            return preloadDistance;
        }

        public boolean isAggressiveUnloading() {
            return aggressiveUnloading;
        }

        public boolean isArenaAllocator() {
            return arenaAllocator;
        }

        public boolean isPoolSmallObjects() {
            return poolSmallObjects;
        }

        // Setters
        public void setOffHeapEnabled(boolean v) {
            this.offHeapEnabled = v;
        }

        public void setOffHeapMaxMB(int v) {
            this.offHeapMaxMB = Math.max(256, v);
        }

        public void setTextureStreaming(boolean v) {
            this.textureStreaming = v;
        }

        public void setTextureBudgetMB(int v) {
            this.textureBudgetMB = Math.max(128, v);
        }

        public void setMeshBudgetMB(int v) {
            this.meshBudgetMB = Math.max(64, v);
        }

        public void setDeduplicationEnabled(boolean v) {
            this.deduplicationEnabled = v;
        }

        public void setGcSyncIntervalMs(int v) {
            this.gcSyncIntervalMs = Math.max(1000, v);
        }

        public void setPreloadChunks(boolean v) {
            this.preloadChunks = v;
        }

        public void setPreloadDistance(int v) {
            this.preloadDistance = Math.max(1, Math.min(v, 8));
        }

        public void setAggressiveUnloading(boolean v) {
            this.aggressiveUnloading = v;
        }

        public void setArenaAllocator(boolean v) {
            this.arenaAllocator = v;
        }

        public void setPoolSmallObjects(boolean v) {
            this.poolSmallObjects = v;
        }
    }

    // =========================================================================
    // NETWORK CONFIGURATION
    // =========================================================================

    /**
     * Network and netcode settings
     */
    public static class NetworkConfig {

        @Expose
        @SerializedName("prediction_enabled")
        private boolean predictionEnabled = true;

        @Expose
        @SerializedName("interpolation_enabled")
        private boolean interpolationEnabled = true;

        @Expose
        @SerializedName("interpolation_delay_ms")
        private int interpolationDelayMs = 100;

        @Expose
        @SerializedName("delta_compression")
        private boolean deltaCompression = true;

        @Expose
        @SerializedName("zstd_compression")
        private boolean zstdCompression = true;

        @Expose
        @SerializedName("zstd_compression_level")
        private int zstdCompressionLevel = 3;

        @Expose
        @SerializedName("max_prediction_ticks")
        private int maxPredictionTicks = 10;

        @Expose
        @SerializedName("rubber_banding_threshold")
        private float rubberBandingThreshold = 2.0f;

        @Expose
        @SerializedName("smooth_correction")
        private boolean smoothCorrection = true;

        @Expose
        @SerializedName("smooth_correction_rate")
        private float smoothCorrectionRate = 0.5f;

        @Expose
        @SerializedName("packet_buffer_size")
        private int packetBufferSize = 64;

        @Expose
        @SerializedName("compression_dictionary")
        private boolean compressionDictionary = true;

        // Getters
        public boolean isPredictionEnabled() {
            return predictionEnabled;
        }

        public boolean isInterpolationEnabled() {
            return interpolationEnabled;
        }

        public int getInterpolationDelayMs() {
            return interpolationDelayMs;
        }

        public boolean isDeltaCompression() {
            return deltaCompression;
        }

        public boolean isZstdCompression() {
            return zstdCompression;
        }

        public int getZstdCompressionLevel() {
            return zstdCompressionLevel;
        }

        public int getMaxPredictionTicks() {
            return maxPredictionTicks;
        }

        public float getRubberBandingThreshold() {
            return rubberBandingThreshold;
        }

        public boolean isSmoothCorrection() {
            return smoothCorrection;
        }

        public float getSmoothCorrectionRate() {
            return smoothCorrectionRate;
        }

        public int getPacketBufferSize() {
            return packetBufferSize;
        }

        public boolean isCompressionDictionary() {
            return compressionDictionary;
        }

        // Setters
        public void setPredictionEnabled(boolean v) {
            this.predictionEnabled = v;
        }

        public void setInterpolationEnabled(boolean v) {
            this.interpolationEnabled = v;
        }

        public void setInterpolationDelayMs(int v) {
            this.interpolationDelayMs = Math.max(0, v);
        }

        public void setDeltaCompression(boolean v) {
            this.deltaCompression = v;
        }

        public void setZstdCompression(boolean v) {
            this.zstdCompression = v;
        }

        public void setZstdCompressionLevel(int v) {
            this.zstdCompressionLevel = Math.max(1, Math.min(v, 22));
        }

        public void setMaxPredictionTicks(int v) {
            this.maxPredictionTicks = Math.max(1, Math.min(v, 60));
        }

        public void setRubberBandingThreshold(float v) {
            this.rubberBandingThreshold = Math.max(0.5f, v);
        }

        public void setSmoothCorrection(boolean v) {
            this.smoothCorrection = v;
        }

        public void setSmoothCorrectionRate(float v) {
            this.smoothCorrectionRate = Math.max(0.1f, Math.min(v, 1.0f));
        }

        public void setPacketBufferSize(int v) {
            this.packetBufferSize = Math.max(16, v);
        }

        public void setCompressionDictionary(boolean v) {
            this.compressionDictionary = v;
        }
    }

    // =========================================================================
    // PHYSICS CONFIGURATION
    // =========================================================================

    /**
     * Physics engine settings
     */
    public static class PhysicsConfig {

        @Expose
        @SerializedName("parallel_ticking")
        private boolean parallelTicking = true;

        @Expose
        @SerializedName("max_physics_threads")
        private int maxPhysicsThreads = 0; // 0 = auto

        @Expose
        @SerializedName("gpu_physics")
        private boolean gpuPhysics = true;

        @Expose
        @SerializedName("explosion_optimization")
        private boolean explosionOptimization = true;

        @Expose
        @SerializedName("entity_batch_size")
        private int entityBatchSize = 256;

        @Expose
        @SerializedName("partition_size")
        private int partitionSize = 16;

        @Expose
        @SerializedName("async_pathfinding")
        private boolean asyncPathfinding = true;

        @Expose
        @SerializedName("collision_optimization")
        private boolean collisionOptimization = true;

        @Expose
        @SerializedName("fluid_simulation")
        private boolean fluidSimulation = true;

        @Expose
        @SerializedName("fluid_quality")
        private QualityLevel fluidQuality = QualityLevel.MEDIUM;

        // Getters
        public boolean isParallelTicking() {
            return parallelTicking;
        }

        public int getMaxPhysicsThreads() {
            return maxPhysicsThreads;
        }

        public boolean isGpuPhysics() {
            return gpuPhysics;
        }

        public boolean isExplosionOptimization() {
            return explosionOptimization;
        }

        public int getEntityBatchSize() {
            return entityBatchSize;
        }

        public int getPartitionSize() {
            return partitionSize;
        }

        public boolean isAsyncPathfinding() {
            return asyncPathfinding;
        }

        public boolean isCollisionOptimization() {
            return collisionOptimization;
        }

        public boolean isFluidSimulation() {
            return fluidSimulation;
        }

        public QualityLevel getFluidQuality() {
            return fluidQuality;
        }

        // Setters
        public void setParallelTicking(boolean v) {
            this.parallelTicking = v;
        }

        public void setMaxPhysicsThreads(int v) {
            this.maxPhysicsThreads = Math.max(0, v);
        }

        public void setGpuPhysics(boolean v) {
            this.gpuPhysics = v;
        }

        public void setExplosionOptimization(boolean v) {
            this.explosionOptimization = v;
        }

        public void setEntityBatchSize(int v) {
            this.entityBatchSize = Math.max(32, v);
        }

        public void setPartitionSize(int v) {
            this.partitionSize = Math.max(8, v);
        }

        public void setAsyncPathfinding(boolean v) {
            this.asyncPathfinding = v;
        }

        public void setCollisionOptimization(boolean v) {
            this.collisionOptimization = v;
        }

        public void setFluidSimulation(boolean v) {
            this.fluidSimulation = v;
        }

        public void setFluidQuality(QualityLevel v) {
            this.fluidQuality = v;
        }
    }

    // =========================================================================
    // AUDIO CONFIGURATION
    // =========================================================================

    /**
     * Audio engine settings
     */
    public static class AudioConfig {

        @Expose
        @SerializedName("raytraced_audio")
        private boolean raytracedAudio = true;

        @Expose
        @SerializedName("occlusion_enabled")
        private boolean occlusionEnabled = true;

        @Expose
        @SerializedName("reverb_enabled")
        private boolean reverbEnabled = true;

        @Expose
        @SerializedName("hrtf_enabled")
        private boolean hrtfEnabled = false;

        @Expose
        @SerializedName("max_sound_sources")
        private int maxSoundSources = 64;

        @Expose
        @SerializedName("audio_quality")
        private QualityLevel audioQuality = QualityLevel.HIGH;

        @Expose
        @SerializedName("doppler_effect")
        private boolean dopplerEffect = true;

        @Expose
        @SerializedName("material_absorption")
        private boolean materialAbsorption = true;

        // Getters
        public boolean isRaytracedAudio() {
            return raytracedAudio;
        }

        public boolean isOcclusionEnabled() {
            return occlusionEnabled;
        }

        public boolean isReverbEnabled() {
            return reverbEnabled;
        }

        public boolean isHrtfEnabled() {
            return hrtfEnabled;
        }

        public int getMaxSoundSources() {
            return maxSoundSources;
        }

        public QualityLevel getAudioQuality() {
            return audioQuality;
        }

        public boolean isDopplerEffect() {
            return dopplerEffect;
        }

        public boolean isMaterialAbsorption() {
            return materialAbsorption;
        }

        // Setters
        public void setRaytracedAudio(boolean v) {
            this.raytracedAudio = v;
        }

        public void setOcclusionEnabled(boolean v) {
            this.occlusionEnabled = v;
        }

        public void setReverbEnabled(boolean v) {
            this.reverbEnabled = v;
        }

        public void setHrtfEnabled(boolean v) {
            this.hrtfEnabled = v;
        }

        public void setMaxSoundSources(int v) {
            this.maxSoundSources = Math.max(16, v);
        }

        public void setAudioQuality(QualityLevel v) {
            this.audioQuality = v;
        }

        public void setDopplerEffect(boolean v) {
            this.dopplerEffect = v;
        }

        public void setMaterialAbsorption(boolean v) {
            this.materialAbsorption = v;
        }
    }

    // =========================================================================
    // COMPATIBILITY CONFIGURATION
    // =========================================================================

    /**
     * Mod compatibility settings
     */
    public static class CompatibilityConfig {

        @Expose
        @SerializedName("auto_fallback")
        private boolean autoFallback = true;

        @Expose
        @SerializedName("legacy_texture_support")
        private boolean legacyTextureSupport = true;

        @Expose
        @SerializedName("force_legacy_mods")
        private Set<String> forceLegacyMods = new HashSet<>();

        @Expose
        @SerializedName("disabled_optimizations")
        private Set<String> disabledOptimizations = new HashSet<>();

        @Expose
        @SerializedName("shader_pack_compat")
        private boolean shaderPackCompat = true;

        @Expose
        @SerializedName("optifine_compat")
        private boolean optifineCompat = true;

        @Expose
        @SerializedName("sodium_compat")
        private boolean sodiumCompat = true;

        @Expose
        @SerializedName("iris_compat")
        private boolean irisCompat = true;

        @Expose
        @SerializedName("mod_profiling")
        private boolean modProfiling = true;

        @Expose
        @SerializedName("crash_recovery")
        private boolean crashRecovery = true;

        // Getters
        public boolean isAutoFallback() {
            return autoFallback;
        }

        public boolean isLegacyTextureSupport() {
            return legacyTextureSupport;
        }

        public Set<String> getForceLegacyMods() {
            return Collections.unmodifiableSet(forceLegacyMods);
        }

        public Set<String> getDisabledOptimizations() {
            return Collections.unmodifiableSet(disabledOptimizations);
        }

        public boolean isShaderPackCompat() {
            return shaderPackCompat;
        }

        public boolean isOptifineCompat() {
            return optifineCompat;
        }

        public boolean isSodiumCompat() {
            return sodiumCompat;
        }

        public boolean isIrisCompat() {
            return irisCompat;
        }

        public boolean isModProfiling() {
            return modProfiling;
        }

        public boolean isCrashRecovery() {
            return crashRecovery;
        }

        // Setters
        public void setAutoFallback(boolean v) {
            this.autoFallback = v;
        }

        public void setLegacyTextureSupport(boolean v) {
            this.legacyTextureSupport = v;
        }

        public void addForceLegacyMod(String modId) {
            this.forceLegacyMods.add(modId);
        }

        public void removeForceLegacyMod(String modId) {
            this.forceLegacyMods.remove(modId);
        }

        public void addDisabledOptimization(String opt) {
            this.disabledOptimizations.add(opt);
        }

        public void removeDisabledOptimization(String opt) {
            this.disabledOptimizations.remove(opt);
        }

        public void setShaderPackCompat(boolean v) {
            this.shaderPackCompat = v;
        }

        public void setOptifineCompat(boolean v) {
            this.optifineCompat = v;
        }

        public void setSodiumCompat(boolean v) {
            this.sodiumCompat = v;
        }

        public void setIrisCompat(boolean v) {
            this.irisCompat = v;
        }

        public void setModProfiling(boolean v) {
            this.modProfiling = v;
        }

        public void setCrashRecovery(boolean v) {
            this.crashRecovery = v;
        }
    }

    // =========================================================================
    // DEBUG CONFIGURATION
    // =========================================================================

    /**
     * Debug and development settings
     */
    public static class DebugConfig {

        @Expose
        @SerializedName("debug_mode")
        private boolean debugMode = false;

        @Expose
        @SerializedName("verbose_logging")
        private boolean verboseLogging = false;

        @Expose
        @SerializedName("show_fps_overlay")
        private boolean showFpsOverlay = false;

        @Expose
        @SerializedName("show_memory_overlay")
        private boolean showMemoryOverlay = false;

        @Expose
        @SerializedName("show_gpu_overlay")
        private boolean showGpuOverlay = false;

        @Expose
        @SerializedName("wireframe_mode")
        private boolean wireframeMode = false;

        @Expose
        @SerializedName("show_chunk_borders")
        private boolean showChunkBorders = false;

        @Expose
        @SerializedName("show_culling_debug")
        private boolean showCullingDebug = false;

        @Expose
        @SerializedName("profiling_enabled")
        private boolean profilingEnabled = false;

        @Expose
        @SerializedName("dump_shaders")
        private boolean dumpShaders = false;

        @Expose
        @SerializedName("validation_layers")
        private boolean validationLayers = false;

        @Expose
        @SerializedName("api_dump")
        private boolean apiDump = false;

        // Getters
        public boolean isDebugMode() {
            return debugMode;
        }

        public boolean isVerboseLogging() {
            return verboseLogging;
        }

        public boolean isShowFpsOverlay() {
            return showFpsOverlay;
        }

        public boolean isShowMemoryOverlay() {
            return showMemoryOverlay;
        }

        public boolean isShowGpuOverlay() {
            return showGpuOverlay;
        }

        public boolean isWireframeMode() {
            return wireframeMode;
        }

        public boolean isShowChunkBorders() {
            return showChunkBorders;
        }

        public boolean isShowCullingDebug() {
            return showCullingDebug;
        }

        public boolean isProfilingEnabled() {
            return profilingEnabled;
        }

        public boolean isDumpShaders() {
            return dumpShaders;
        }

        public boolean isValidationLayers() {
            return validationLayers;
        }

        public boolean isApiDump() {
            return apiDump;
        }

        // Setters
        public void setDebugMode(boolean v) {
            this.debugMode = v;
        }

        public void setVerboseLogging(boolean v) {
            this.verboseLogging = v;
        }

        public void setShowFpsOverlay(boolean v) {
            this.showFpsOverlay = v;
        }

        public void setShowMemoryOverlay(boolean v) {
            this.showMemoryOverlay = v;
        }

        public void setShowGpuOverlay(boolean v) {
            this.showGpuOverlay = v;
        }

        public void setWireframeMode(boolean v) {
            this.wireframeMode = v;
        }

        public void setShowChunkBorders(boolean v) {
            this.showChunkBorders = v;
        }

        public void setShowCullingDebug(boolean v) {
            this.showCullingDebug = v;
        }

        public void setProfilingEnabled(boolean v) {
            this.profilingEnabled = v;
        }

        public void setDumpShaders(boolean v) {
            this.dumpShaders = v;
        }

        public void setValidationLayers(boolean v) {
            this.validationLayers = v;
        }

        public void setApiDump(boolean v) {
            this.apiDump = v;
        }
    }

    // =========================================================================
    // FACTORY METHODS
    // =========================================================================

    /**
     * Create default configuration
     */
    @NotNull
    public static LibsConfig createDefault() {
        return new LibsConfig();
    }

    /**
     * Load configuration from file
     */
    @NotNull
    public static LibsConfig load(@NotNull Path path) throws IOException {
        Objects.requireNonNull(path, "path cannot be null");

        if (!Files.exists(path)) {
            LOGGER.info("Config file not found, creating default: {}", path);
            LibsConfig config = createDefault();
            config.configPath = path;
            config.save();
            return config;
        }

        try (BufferedReader reader = Files.newBufferedReader(path, StandardCharsets.UTF_8)) {
            LibsConfig config = GSON.fromJson(reader, LibsConfig.class);
            if (config == null) {
                config = createDefault();
            }
            config.configPath = path;
            config.lastModifiedTime = Files.getLastModifiedTime(path).toMillis();

            // Migrate schema if needed
            if (config.schemaVersion < SCHEMA_VERSION) {
                config.migrateSchema();
                config.save();
            }

            return config;
        }
    }

    /**
     * Save configuration to file
     */
    public void save() throws IOException {
        if (configPath == null) {
            throw new IllegalStateException("Config path not set");
        }
        save(configPath);
    }

    /**
     * Save configuration to specified file
     */
    public void save(@NotNull Path path) throws IOException {
        Objects.requireNonNull(path, "path cannot be null");

        // Ensure parent directories exist
        Path parent = path.getParent();
        if (parent != null && !Files.exists(parent)) {
            Files.createDirectories(parent);
        }

        try (BufferedWriter writer = Files.newBufferedWriter(path, StandardCharsets.UTF_8,
                StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING)) {
            GSON.toJson(this, writer);
        }

        this.configPath = path;
        this.lastModifiedTime = System.currentTimeMillis();
        LOGGER.info("Configuration saved to: {}", path);
    }

    /**
     * Migrate configuration from older schema version
     */
    private void migrateSchema() {
        LOGGER.info("Migrating config from schema v{} to v{}", schemaVersion, SCHEMA_VERSION);

        // Add migration logic here for future schema changes
        // Example:
        // if (schemaVersion < 2) {
        // // Migrate from v1 to v2
        // }

        schemaVersion = SCHEMA_VERSION;
    }

    /**
     * Check if config file has been modified
     */
    public boolean hasFileChanged() {
        if (configPath == null || !Files.exists(configPath)) {
            return false;
        }
        try {
            return Files.getLastModifiedTime(configPath).toMillis() > lastModifiedTime;
        } catch (IOException e) {
            return false;
        }
    }

    /**
     * Reload configuration from file
     */
    public void reload() throws IOException {
        if (configPath == null) {
            throw new IllegalStateException("Config path not set");
        }

        LibsConfig reloaded = load(configPath);

        // Copy values
        this.general = reloaded.general;
        this.render = reloaded.render;
        this.memory = reloaded.memory;
        this.network = reloaded.network;
        this.physics = reloaded.physics;
        this.audio = reloaded.audio;
        this.compatibility = reloaded.compatibility;
        this.debug = reloaded.debug;
        this.lastModifiedTime = reloaded.lastModifiedTime;

        // Notify listeners
        notifyListeners();
    }

    // =========================================================================
    // LISTENERS
    // =========================================================================

    /**
     * Add configuration change listener
     */
    public void addChangeListener(Consumer<LibsConfig> listener) {
        changeListeners.add(listener);
    }

    /**
     * Remove configuration change listener
     */
    public void removeChangeListener(Consumer<LibsConfig> listener) {
        changeListeners.remove(listener);
    }

    /**
     * Notify all change listeners
     */
    private void notifyListeners() {
        for (Consumer<LibsConfig> listener : changeListeners) {
            try {
                listener.accept(this);
            } catch (Exception e) {
                LOGGER.error("Config listener error: {}", e.getMessage());
            }
        }
    }

    // =========================================================================
    // GETTERS
    // =========================================================================

    public int getSchemaVersion() {
        return schemaVersion;
    }

    public GeneralConfig getGeneral() {
        return general;
    }

    public RenderConfig getRender() {
        return render;
    }

    public PhysicsConfig getPhysics() {
        return physics;
    }

    public AudioConfig getAudio() {
        return audio;
    }

    public CompatibilityConfig getCompatibility() {
        return compatibility;
    }

    public DebugConfig getDebug() {
        return debug;
    }

    public Path getConfigPath() {
        return configPath;
    }

    // Convenience methods
    public boolean isEnabled() {
        return general.isEnabled();
    }

    public boolean isForceSafeMode() {
        return general.isForceSafeMode();
    }

    public boolean isVulkanEnabled() {
        return render.isVulkanEnabled();
    }

    public boolean isHybridModePreferred() {
        return render.isHybridMode();
    }

    // =========================================================================
    // NATIVE CONVERSION
    // =========================================================================

    /**
     * Convert configuration to native format for JNI
     * 
     * @return byte array containing serialized configuration
     */
    public byte[] toNativeFormat() {
        // Serialize to flattened binary format matching Rust's EngineConfig
        JsonObject json = new JsonObject();

        // Render Mode (Enum mapping)
        String renderMode = "HYBRID";
        if (render.isVulkanEnabled()) {
            if (render.isHybridMode()) {
                renderMode = "HYBRID";
            } else {
                renderMode = "VULKAN";
            }
        } else {
            renderMode = "OPENGL";
        }
        json.addProperty("renderMode", renderMode);

        // Memory
        json.addProperty("maxOffheapMB", memory.getOffHeapMaxMB());

        // Render Settings
        json.addProperty("vsync", render.isVsync());
        json.addProperty("maxFps", render.getTargetFps());
        json.addProperty("renderScale", render.getResolutionScale());

        // These fields need to be added to RenderConfig or hardcoded/mapped if missing
        // For now mapping closest available or defaults
        json.addProperty("asyncChunks", true); // Default
        json.addProperty("meshThreads", 4); // Default

        // Debug
        json.addProperty("validationLayers", debug.isValidationLayers());
        json.addProperty("ecsProfiling", debug.isProfilingEnabled());

        // Audio
        json.addProperty("masterVolume", 1.0f); // Default, maybe add to AudioConfig later

        return json.toString().getBytes(StandardCharsets.UTF_8);
    }
}
