/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * HardwareProfile.java - Hardware Profile Data Class
 * 
 * Contains all detected hardware information in a single immutable object.
 */

package dev.libs.hardware;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * HardwareProfile - Immutable Hardware Information Container
 * 
 * Contains all detected system hardware information including:
 * - Operating system details
 * - CPU model, cores, and features
 * - GPU model, vendor, and memory
 * - System memory information
 * - Java runtime information
 * - Hardware hash for identification
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class HardwareProfile {

    // =========================================================================
    // ENUMS
    // =========================================================================

    public enum GPUType {
        NVIDIA,
        AMD,
        INTEL,
        APPLE,
        UNKNOWN
    }

    // =========================================================================
    // FIELDS - OS
    // =========================================================================

    private final HardwareDetector.OSType osType;
    private final String osName;
    private final String osVersion;
    private final HardwareDetector.Architecture architecture;

    // =========================================================================
    // FIELDS - CPU
    // =========================================================================

    private final String cpuName;
    private final int cpuCores;
    private final List<String> cpuFeatures;
    private final boolean sseSupported;
    private final boolean avxSupported;
    private final boolean avx2Supported;
    private final boolean avx512Supported;

    // =========================================================================
    // FIELDS - GPU
    // =========================================================================

    private final String gpuName;
    private final String gpuVendor;
    private final GPUType gpuType;
    private final long gpuMemoryMB;

    // =========================================================================
    // FIELDS - MEMORY
    // =========================================================================

    private final long totalMemoryMB;
    private final long availableMemoryMB;
    private final long maxJavaHeapMB;

    // =========================================================================
    // FIELDS - JAVA
    // =========================================================================

    private final String javaVersion;
    private final String javaVendor;
    private final String javaHome;
    private final String jvmName;
    private final String jvmVersion;

    // =========================================================================
    // FIELDS - IDENTIFICATION
    // =========================================================================

    private final String hardwareHash;

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    private HardwareProfile(Builder builder) {
        this.osType = builder.osType;
        this.osName = builder.osName;
        this.osVersion = builder.osVersion;
        this.architecture = builder.architecture;

        this.cpuName = builder.cpuName;
        this.cpuCores = builder.cpuCores;
        this.cpuFeatures = Collections.unmodifiableList(new ArrayList<>(builder.cpuFeatures));
        this.sseSupported = builder.sseSupported;
        this.avxSupported = builder.avxSupported;
        this.avx2Supported = builder.avx2Supported;
        this.avx512Supported = builder.avx512Supported;

        this.gpuName = builder.gpuName;
        this.gpuVendor = builder.gpuVendor;
        this.gpuType = builder.gpuType;
        this.gpuMemoryMB = builder.gpuMemoryMB;

        this.totalMemoryMB = builder.totalMemoryMB;
        this.availableMemoryMB = builder.availableMemoryMB;
        this.maxJavaHeapMB = builder.maxJavaHeapMB;

        this.javaVersion = builder.javaVersion;
        this.javaVendor = builder.javaVendor;
        this.javaHome = builder.javaHome;
        this.jvmName = builder.jvmName;
        this.jvmVersion = builder.jvmVersion;

        this.hardwareHash = builder.hardwareHash;
    }

    // =========================================================================
    // CAPABILITY CHECKS
    // =========================================================================

    /**
     * Check if this hardware supports Vulkan rendering
     */
    public boolean supportsVulkan() {
        // Vulkan requires:
        // - 64-bit architecture
        // - Modern GPU (not too old)
        // - Sufficient GPU memory

        if (architecture != HardwareDetector.Architecture.X86_64 &&
                architecture != HardwareDetector.Architecture.ARM64) {
            return false;
        }

        if (gpuType == GPUType.UNKNOWN) {
            return false;
        }

        // Intel integrated graphics before Gen 7 don't support Vulkan well
        if (gpuType == GPUType.INTEL) {
            String gpuLower = gpuName.toLowerCase();
            if (gpuLower.contains("hd graphics") || gpuLower.contains("hd 4000") ||
                    gpuLower.contains("hd 3000") || gpuLower.contains("hd 2000")) {
                return false;
            }
        }

        return true;
    }

    /**
     * Check if this hardware supports ray tracing
     */
    public boolean supportsRayTracing() {
        String gpuLower = gpuName.toLowerCase();

        // NVIDIA RTX series
        if (gpuType == GPUType.NVIDIA) {
            return gpuLower.contains("rtx") ||
                    gpuLower.contains("titan v") ||
                    gpuLower.contains("titan rtx");
        }

        // AMD RDNA2+ (RX 6000 series and newer)
        if (gpuType == GPUType.AMD) {
            return gpuLower.contains("rx 6") || gpuLower.contains("rx 7");
        }

        // Intel Arc
        if (gpuType == GPUType.INTEL) {
            return gpuLower.contains("arc");
        }

        // Apple M1 Pro/Max/Ultra and M2/M3 series
        if (gpuType == GPUType.APPLE) {
            return gpuLower.contains("pro") || gpuLower.contains("max") ||
                    gpuLower.contains("ultra") || gpuLower.contains("m2") ||
                    gpuLower.contains("m3");
        }

        return false;
    }

    /**
     * Check if this hardware supports mesh shaders
     */
    public boolean supportsMeshShaders() {
        String gpuLower = gpuName.toLowerCase();

        // NVIDIA Turing and newer (RTX 20xx, 30xx, 40xx)
        if (gpuType == GPUType.NVIDIA) {
            return gpuLower.contains("rtx") || gpuLower.contains("1650") ||
                    gpuLower.contains("1660");
        }

        // AMD RDNA2+ (RX 6000 series and newer)
        if (gpuType == GPUType.AMD) {
            return gpuLower.contains("rx 6") || gpuLower.contains("rx 7");
        }

        // Intel Arc
        if (gpuType == GPUType.INTEL) {
            return gpuLower.contains("arc");
        }

        return false;
    }

    /**
     * Get recommended render distance based on hardware
     */
    public int getRecommendedRenderDistance() {
        // Base on GPU memory and type
        if (gpuMemoryMB >= 8192) {
            return 32;
        } else if (gpuMemoryMB >= 4096) {
            return 24;
        } else if (gpuMemoryMB >= 2048) {
            return 16;
        } else {
            return 12;
        }
    }

    /**
     * Get recommended memory allocation in MB
     */
    public long getRecommendedOffHeapMB() {
        // Use up to 50% of available system memory, max 8 GB
        long recommended = availableMemoryMB / 2;
        return Math.min(recommended, 8192);
    }

    /**
     * Get a performance score (0-100)
     */
    public int getPerformanceScore() {
        int score = 0;

        // CPU cores (max 20 points)
        score += Math.min(cpuCores * 2, 20);

        // CPU features (max 20 points)
        if (avx512Supported)
            score += 20;
        else if (avx2Supported)
            score += 15;
        else if (avxSupported)
            score += 10;
        else if (sseSupported)
            score += 5;

        // GPU type (max 30 points)
        switch (gpuType) {
            case NVIDIA:
                score += 30;
                break;
            case AMD:
                score += 28;
                break;
            case APPLE:
                score += 25;
                break;
            case INTEL:
                score += 15;
                break;
            default:
                score += 5;
        }

        // GPU memory (max 20 points)
        if (gpuMemoryMB >= 16384)
            score += 20;
        else if (gpuMemoryMB >= 8192)
            score += 16;
        else if (gpuMemoryMB >= 4096)
            score += 12;
        else if (gpuMemoryMB >= 2048)
            score += 8;
        else
            score += 4;

        // System memory (max 10 points)
        if (totalMemoryMB >= 32768)
            score += 10;
        else if (totalMemoryMB >= 16384)
            score += 8;
        else if (totalMemoryMB >= 8192)
            score += 5;
        else
            score += 2;

        return Math.min(score, 100);
    }

    // =========================================================================
    // SERIALIZATION
    // =========================================================================

    private static final Gson GSON = new GsonBuilder().create();

    /**
     * Convert to JSON string
     */
    public String toJson() {
        return GSON.toJson(this);
    }

    /**
     * Convert to native format (byte array)
     */
    public byte[] toNativeFormat() {
        return toJson().getBytes(StandardCharsets.UTF_8);
    }

    /**
     * Parse from JSON
     */
    public static HardwareProfile fromJson(String json) {
        return GSON.fromJson(json, HardwareProfile.class);
    }

    // =========================================================================
    // GETTERS
    // =========================================================================

    public HardwareDetector.OSType getOsType() {
        return osType;
    }

    public String getOsName() {
        return osName;
    }

    public String getOsVersion() {
        return osVersion;
    }

    public HardwareDetector.Architecture getArchitecture() {
        return architecture;
    }

    public String getCpuName() {
        return cpuName;
    }

    public int getCpuCores() {
        return cpuCores;
    }

    public List<String> getCpuFeatures() {
        return cpuFeatures;
    }

    public boolean isSseSupported() {
        return sseSupported;
    }

    public boolean isAvxSupported() {
        return avxSupported;
    }

    public boolean isAvx2Supported() {
        return avx2Supported;
    }

    public boolean isAvx512Supported() {
        return avx512Supported;
    }

    public String getGpuName() {
        return gpuName;
    }

    public String getGpuVendor() {
        return gpuVendor;
    }

    public GPUType getGpuType() {
        return gpuType;
    }

    public long getGpuMemoryMB() {
        return gpuMemoryMB;
    }

    public long getTotalMemoryMB() {
        return totalMemoryMB;
    }

    public long getAvailableMemoryMB() {
        return availableMemoryMB;
    }

    public long getMaxJavaHeapMB() {
        return maxJavaHeapMB;
    }

    public String getJavaVersion() {
        return javaVersion;
    }

    public String getJavaVendor() {
        return javaVendor;
    }

    public String getJavaHome() {
        return javaHome;
    }

    public String getJvmName() {
        return jvmName;
    }

    public String getJvmVersion() {
        return jvmVersion;
    }

    public String getHardwareHash() {
        return hardwareHash;
    }

    // =========================================================================
    // TO STRING
    // =========================================================================

    @Override
    public String toString() {
        return String.format("HardwareProfile[%s %s, CPU=%s (%d cores), GPU=%s (%d MB), RAM=%d MB, Hash=%s]",
                osName, architecture, cpuName, cpuCores, gpuName, gpuMemoryMB, totalMemoryMB, hardwareHash);
    }

    // =========================================================================
    // BUILDER
    // =========================================================================

    public static class Builder {
        private HardwareDetector.OSType osType = HardwareDetector.OSType.UNKNOWN;
        private String osName = "Unknown";
        private String osVersion = "Unknown";
        private HardwareDetector.Architecture architecture = HardwareDetector.Architecture.UNKNOWN;

        private String cpuName = "Unknown CPU";
        private int cpuCores = 1;
        private List<String> cpuFeatures = new ArrayList<>();
        private boolean sseSupported = false;
        private boolean avxSupported = false;
        private boolean avx2Supported = false;
        private boolean avx512Supported = false;

        private String gpuName = "Unknown GPU";
        private String gpuVendor = "Unknown";
        private GPUType gpuType = GPUType.UNKNOWN;
        private long gpuMemoryMB = 0;

        private long totalMemoryMB = 0;
        private long availableMemoryMB = 0;
        private long maxJavaHeapMB = 0;

        private String javaVersion = "Unknown";
        private String javaVendor = "Unknown";
        private String javaHome = "";
        private String jvmName = "Unknown";
        private String jvmVersion = "Unknown";

        private String hardwareHash = "00000000";

        public Builder setOsType(HardwareDetector.OSType osType) {
            this.osType = osType;
            return this;
        }

        public Builder setOsName(String osName) {
            this.osName = osName;
            return this;
        }

        public Builder setOsVersion(String osVersion) {
            this.osVersion = osVersion;
            return this;
        }

        public Builder setArchitecture(HardwareDetector.Architecture arch) {
            this.architecture = arch;
            return this;
        }

        public Builder setCpuName(String cpuName) {
            this.cpuName = cpuName;
            return this;
        }

        public Builder setCpuCores(int cores) {
            this.cpuCores = cores;
            return this;
        }

        public Builder setCpuFeatures(List<String> features) {
            this.cpuFeatures = features;
            return this;
        }

        public Builder setSseSupported(boolean supported) {
            this.sseSupported = supported;
            return this;
        }

        public Builder setAvxSupported(boolean supported) {
            this.avxSupported = supported;
            return this;
        }

        public Builder setAvx2Supported(boolean supported) {
            this.avx2Supported = supported;
            return this;
        }

        public Builder setAvx512Supported(boolean supported) {
            this.avx512Supported = supported;
            return this;
        }

        public Builder setGpuName(String gpuName) {
            this.gpuName = gpuName;
            return this;
        }

        public Builder setGpuVendor(String gpuVendor) {
            this.gpuVendor = gpuVendor;
            return this;
        }

        public Builder setGpuType(GPUType gpuType) {
            this.gpuType = gpuType;
            return this;
        }

        public Builder setGpuMemoryMB(long gpuMemoryMB) {
            this.gpuMemoryMB = gpuMemoryMB;
            return this;
        }

        public Builder setTotalMemoryMB(long totalMemoryMB) {
            this.totalMemoryMB = totalMemoryMB;
            return this;
        }

        public Builder setAvailableMemoryMB(long availableMemoryMB) {
            this.availableMemoryMB = availableMemoryMB;
            return this;
        }

        public Builder setMaxJavaHeapMB(long maxJavaHeapMB) {
            this.maxJavaHeapMB = maxJavaHeapMB;
            return this;
        }

        public Builder setJavaVersion(String javaVersion) {
            this.javaVersion = javaVersion;
            return this;
        }

        public Builder setJavaVendor(String javaVendor) {
            this.javaVendor = javaVendor;
            return this;
        }

        public Builder setJavaHome(String javaHome) {
            this.javaHome = javaHome;
            return this;
        }

        public Builder setJvmName(String jvmName) {
            this.jvmName = jvmName;
            return this;
        }

        public Builder setJvmVersion(String jvmVersion) {
            this.jvmVersion = jvmVersion;
            return this;
        }

        public Builder setHardwareHash(String hardwareHash) {
            this.hardwareHash = hardwareHash;
            return this;
        }

        // For hash generation
        String getCpuName() {
            return cpuName;
        }

        int getCpuCores() {
            return cpuCores;
        }

        String getGpuName() {
            return gpuName;
        }

        long getTotalMemoryMB() {
            return totalMemoryMB;
        }

        public HardwareProfile build() {
            return new HardwareProfile(this);
        }
    }
}
