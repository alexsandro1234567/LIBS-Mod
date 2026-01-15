/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * VulkanCapabilities.java - Vulkan GPU Capability Detection
 * 
 * Detects Vulkan support and capabilities on the system.
 */

package dev.libs.hardware;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * VulkanCapabilities - Vulkan GPU Feature Detection
 * 
 * Detects Vulkan API support and GPU capabilities including:
 * - Vulkan version
 * - Device properties
 * - Extension support
 * - Compute shader capabilities
 * - Ray tracing support
 * - Memory types
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class VulkanCapabilities {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(VulkanCapabilities.class);

    /** Important Vulkan extensions */
    public static final String EXT_SWAPCHAIN = "VK_KHR_swapchain";
    public static final String EXT_RAY_TRACING = "VK_KHR_ray_tracing_pipeline";
    public static final String EXT_ACCELERATION_STRUCTURE = "VK_KHR_acceleration_structure";
    public static final String EXT_MESH_SHADER = "VK_EXT_mesh_shader";
    public static final String EXT_DESCRIPTOR_INDEXING = "VK_EXT_descriptor_indexing";
    public static final String EXT_BUFFER_DEVICE_ADDRESS = "VK_KHR_buffer_device_address";
    public static final String EXT_DYNAMIC_RENDERING = "VK_KHR_dynamic_rendering";
    public static final String EXT_SYNCHRONIZATION2 = "VK_KHR_synchronization2";
    public static final String EXT_MAINTENANCE4 = "VK_KHR_maintenance4";

    // =========================================================================
    // FIELDS
    // =========================================================================

    private final boolean available;
    private final int versionMajor;
    private final int versionMinor;
    private final int versionPatch;
    private final String deviceName;
    private final String driverVersion;
    private final int vendorId;
    private final int deviceId;
    private final DeviceType deviceType;
    private final List<String> extensions;
    private final List<String> layers;

    // Limits
    private final int maxImageDimension2D;
    private final int maxImageDimension3D;
    private final int maxImageArrayLayers;
    private final int maxUniformBufferRange;
    private final int maxStorageBufferRange;
    private final int maxPushConstantsSize;
    private final int maxMemoryAllocationCount;
    private final int maxBoundDescriptorSets;
    private final int maxComputeSharedMemorySize;
    private final int maxComputeWorkGroupInvocations;
    private final int[] maxComputeWorkGroupCount;
    private final int[] maxComputeWorkGroupSize;

    // Features
    private final boolean geometryShader;
    private final boolean tessellationShader;
    private final boolean multiDrawIndirect;
    private final boolean drawIndirectFirstInstance;
    private final boolean depthClamp;
    private final boolean depthBiasClamp;
    private final boolean fillModeNonSolid;
    private final boolean wideLines;
    private final boolean samplerAnisotropy;
    private final boolean textureCompressionBC;
    private final boolean shaderFloat64;
    private final boolean shaderInt64;
    private final boolean shaderInt16;

    // Advanced features
    private final boolean computeSupported;
    private final boolean rayTracingSupported;
    private final boolean meshShaderSupported;
    private final boolean bindlessSupported;

    // Memory
    private final long deviceLocalMemoryMB;
    private final long hostVisibleMemoryMB;

    // =========================================================================
    // ENUMS
    // =========================================================================

    public enum DeviceType {
        OTHER,
        INTEGRATED_GPU,
        DISCRETE_GPU,
        VIRTUAL_GPU,
        CPU
    }

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    private VulkanCapabilities(Builder builder) {
        this.available = builder.available;
        this.versionMajor = builder.versionMajor;
        this.versionMinor = builder.versionMinor;
        this.versionPatch = builder.versionPatch;
        this.deviceName = builder.deviceName;
        this.driverVersion = builder.driverVersion;
        this.vendorId = builder.vendorId;
        this.deviceId = builder.deviceId;
        this.deviceType = builder.deviceType;
        this.extensions = Collections.unmodifiableList(new ArrayList<>(builder.extensions));
        this.layers = Collections.unmodifiableList(new ArrayList<>(builder.layers));

        this.maxImageDimension2D = builder.maxImageDimension2D;
        this.maxImageDimension3D = builder.maxImageDimension3D;
        this.maxImageArrayLayers = builder.maxImageArrayLayers;
        this.maxUniformBufferRange = builder.maxUniformBufferRange;
        this.maxStorageBufferRange = builder.maxStorageBufferRange;
        this.maxPushConstantsSize = builder.maxPushConstantsSize;
        this.maxMemoryAllocationCount = builder.maxMemoryAllocationCount;
        this.maxBoundDescriptorSets = builder.maxBoundDescriptorSets;
        this.maxComputeSharedMemorySize = builder.maxComputeSharedMemorySize;
        this.maxComputeWorkGroupInvocations = builder.maxComputeWorkGroupInvocations;
        this.maxComputeWorkGroupCount = builder.maxComputeWorkGroupCount.clone();
        this.maxComputeWorkGroupSize = builder.maxComputeWorkGroupSize.clone();

        this.geometryShader = builder.geometryShader;
        this.tessellationShader = builder.tessellationShader;
        this.multiDrawIndirect = builder.multiDrawIndirect;
        this.drawIndirectFirstInstance = builder.drawIndirectFirstInstance;
        this.depthClamp = builder.depthClamp;
        this.depthBiasClamp = builder.depthBiasClamp;
        this.fillModeNonSolid = builder.fillModeNonSolid;
        this.wideLines = builder.wideLines;
        this.samplerAnisotropy = builder.samplerAnisotropy;
        this.textureCompressionBC = builder.textureCompressionBC;
        this.shaderFloat64 = builder.shaderFloat64;
        this.shaderInt64 = builder.shaderInt64;
        this.shaderInt16 = builder.shaderInt16;

        this.computeSupported = builder.computeSupported;
        this.rayTracingSupported = builder.rayTracingSupported;
        this.meshShaderSupported = builder.meshShaderSupported;
        this.bindlessSupported = builder.bindlessSupported;

        this.deviceLocalMemoryMB = builder.deviceLocalMemoryMB;
        this.hostVisibleMemoryMB = builder.hostVisibleMemoryMB;
    }

    // =========================================================================
    // DETECTION
    // =========================================================================

    /**
     * Detect Vulkan capabilities on this system
     * 
     * @return VulkanCapabilities instance
     * @throws RuntimeException if Vulkan is not available
     */
    @NotNull
    public static VulkanCapabilities detect() {
        LOGGER.info("Detecting Vulkan capabilities...");

        Builder builder = new Builder();

        // Try to run vulkaninfo
        if (!detectViaVulkanInfo(builder)) {
            // Vulkaninfo not available, try alternative methods
            if (!detectViaSystemCheck(builder)) {
                // No Vulkan detected
                LOGGER.warn("Vulkan not available on this system");
                builder.setAvailable(false);
                return builder.build();
            }
        }

        builder.setAvailable(true);

        // Determine advanced feature support based on extensions
        builder.setRayTracingSupported(
                builder.extensions.contains(EXT_RAY_TRACING) ||
                        builder.extensions.contains(EXT_ACCELERATION_STRUCTURE));

        builder.setMeshShaderSupported(
                builder.extensions.contains(EXT_MESH_SHADER));

        builder.setBindlessSupported(
                builder.extensions.contains(EXT_DESCRIPTOR_INDEXING) ||
                        builder.versionMinor >= 2 // Vulkan 1.2+ has this built-in
        );

        VulkanCapabilities caps = builder.build();

        LOGGER.info("Vulkan detection complete:");
        LOGGER.info("  Version: {}", caps.getVersionString());
        LOGGER.info("  Device: {} ({})", caps.deviceName, caps.deviceType);
        LOGGER.info("  Extensions: {}", caps.extensions.size());
        LOGGER.info("  Ray Tracing: {}", caps.rayTracingSupported ? "Yes" : "No");
        LOGGER.info("  Mesh Shaders: {}", caps.meshShaderSupported ? "Yes" : "No");
        LOGGER.info("  Compute: {}", caps.computeSupported ? "Yes" : "No");

        return caps;
    }

    /**
     * Detect via vulkaninfo command
     */
    private static boolean detectViaVulkanInfo(Builder builder) {
        try {
            ProcessBuilder pb = new ProcessBuilder("vulkaninfo", "--summary");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {

                String line;
                boolean inDeviceProperties = false;

                while ((line = reader.readLine()) != null) {
                    line = line.trim();

                    // Parse Vulkan version
                    if (line.contains("apiVersion")) {
                        parseVersion(line, builder);
                    }

                    // Parse device name
                    if (line.contains("deviceName")) {
                        int eqIdx = line.indexOf('=');
                        if (eqIdx > 0) {
                            builder.setDeviceName(line.substring(eqIdx + 1).trim());
                        }
                    }

                    // Parse device type
                    if (line.contains("deviceType")) {
                        if (line.contains("DISCRETE")) {
                            builder.setDeviceType(DeviceType.DISCRETE_GPU);
                        } else if (line.contains("INTEGRATED")) {
                            builder.setDeviceType(DeviceType.INTEGRATED_GPU);
                        } else if (line.contains("VIRTUAL")) {
                            builder.setDeviceType(DeviceType.VIRTUAL_GPU);
                        } else if (line.contains("CPU")) {
                            builder.setDeviceType(DeviceType.CPU);
                        }
                    }

                    // Parse driver version
                    if (line.contains("driverVersion")) {
                        int eqIdx = line.indexOf('=');
                        if (eqIdx > 0) {
                            builder.setDriverVersion(line.substring(eqIdx + 1).trim());
                        }
                    }

                    // Parse max image dimension
                    if (line.contains("maxImageDimension2D")) {
                        try {
                            int eqIdx = line.indexOf('=');
                            if (eqIdx > 0) {
                                builder.setMaxImageDimension2D(
                                        Integer.parseInt(line.substring(eqIdx + 1).trim()));
                            }
                        } catch (NumberFormatException ignored) {
                        }
                    }
                }

                int exitCode = process.waitFor();
                return exitCode == 0 && builder.deviceName != null;
            }
        } catch (Exception e) {
            LOGGER.debug("vulkaninfo not available: {}", e.getMessage());
            return false;
        }
    }

    /**
     * Parse Vulkan version string (e.g., "1.3.250")
     */
    private static void parseVersion(String line, Builder builder) {
        Pattern pattern = Pattern.compile("(\\d+)\\.(\\d+)\\.(\\d+)");
        Matcher matcher = pattern.matcher(line);
        if (matcher.find()) {
            try {
                builder.setVersionMajor(Integer.parseInt(matcher.group(1)));
                builder.setVersionMinor(Integer.parseInt(matcher.group(2)));
                builder.setVersionPatch(Integer.parseInt(matcher.group(3)));
            } catch (NumberFormatException ignored) {
            }
        }
    }

    /**
     * Fallback system check for Vulkan
     */
    private static boolean detectViaSystemCheck(Builder builder) {
        // Check for Vulkan DLL/SO presence
        String osName = System.getProperty("os.name", "").toLowerCase();

        if (osName.contains("windows")) {
            // Check for vulkan-1.dll
            String[] paths = {
                    System.getenv("WINDIR") + "\\System32\\vulkan-1.dll",
                    System.getenv("WINDIR") + "\\SysWOW64\\vulkan-1.dll"
            };

            for (String path : paths) {
                if (new java.io.File(path).exists()) {
                    // Vulkan runtime exists, assume basic support
                    builder.setVersionMajor(1);
                    builder.setVersionMinor(0);
                    builder.setDeviceName("Unknown Vulkan Device");
                    builder.setComputeSupported(true);
                    return true;
                }
            }
        } else if (osName.contains("linux")) {
            // Check for libvulkan.so
            String[] paths = {
                    "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
                    "/usr/lib/libvulkan.so.1",
                    "/usr/lib64/libvulkan.so.1"
            };

            for (String path : paths) {
                if (new java.io.File(path).exists()) {
                    builder.setVersionMajor(1);
                    builder.setVersionMinor(0);
                    builder.setDeviceName("Unknown Vulkan Device");
                    builder.setComputeSupported(true);
                    return true;
                }
            }
        } else if (osName.contains("mac")) {
            // macOS uses MoltenVK
            // Check if MoltenVK is available
            String[] paths = {
                    "/usr/local/lib/libMoltenVK.dylib",
                    "/opt/homebrew/lib/libMoltenVK.dylib"
            };

            for (String path : paths) {
                if (new java.io.File(path).exists()) {
                    builder.setVersionMajor(1);
                    builder.setVersionMinor(2); // MoltenVK typically supports 1.2
                    builder.setDeviceName("MoltenVK");
                    builder.setComputeSupported(true);
                    return true;
                }
            }
        }

        return false;
    }

    /**
     * Create a minimal capabilities object for when Vulkan is unavailable
     */
    public static VulkanCapabilities unavailable() {
        return new Builder().setAvailable(false).build();
    }

    // =========================================================================
    // CAPABILITY CHECKS
    // =========================================================================

    /**
     * Check if Vulkan is available
     */
    public boolean isAvailable() {
        return available;
    }

    /**
     * Check if all required extensions for Libs are supported
     */
    public boolean isFullySupported() {
        if (!available)
            return false;

        // Minimum requirements
        if (versionMajor < 1)
            return false;
        if (versionMajor == 1 && versionMinor < 1)
            return false;

        // Must have swapchain
        if (!extensions.contains(EXT_SWAPCHAIN))
            return false;

        // Must have compute
        if (!computeSupported)
            return false;

        return true;
    }

    /**
     * Check if an extension is supported
     */
    public boolean hasExtension(String extension) {
        return extensions.contains(extension);
    }

    /**
     * Get the Vulkan version string
     */
    public String getVersionString() {
        return String.format("%d.%d.%d", versionMajor, versionMinor, versionPatch);
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

    // =========================================================================
    // GETTERS
    // =========================================================================

    public int getVersionMajor() {
        return versionMajor;
    }

    public int getVersionMinor() {
        return versionMinor;
    }

    public int getVersionPatch() {
        return versionPatch;
    }

    public String getDeviceName() {
        return deviceName;
    }

    public String getDriverVersion() {
        return driverVersion;
    }

    public int getVendorId() {
        return vendorId;
    }

    public int getDeviceId() {
        return deviceId;
    }

    public DeviceType getDeviceType() {
        return deviceType;
    }

    public List<String> getExtensions() {
        return extensions;
    }

    public List<String> getLayers() {
        return layers;
    }

    public int getMaxImageDimension2D() {
        return maxImageDimension2D;
    }

    public int getMaxImageDimension3D() {
        return maxImageDimension3D;
    }

    public int getMaxImageArrayLayers() {
        return maxImageArrayLayers;
    }

    public int getMaxUniformBufferRange() {
        return maxUniformBufferRange;
    }

    public int getMaxStorageBufferRange() {
        return maxStorageBufferRange;
    }

    public int getMaxPushConstantsSize() {
        return maxPushConstantsSize;
    }

    public int getMaxMemoryAllocationCount() {
        return maxMemoryAllocationCount;
    }

    public int getMaxBoundDescriptorSets() {
        return maxBoundDescriptorSets;
    }

    public int getMaxComputeSharedMemorySize() {
        return maxComputeSharedMemorySize;
    }

    public int getMaxComputeWorkGroupInvocations() {
        return maxComputeWorkGroupInvocations;
    }

    public int[] getMaxComputeWorkGroupCount() {
        return maxComputeWorkGroupCount.clone();
    }

    public int[] getMaxComputeWorkGroupSize() {
        return maxComputeWorkGroupSize.clone();
    }

    public boolean isGeometryShader() {
        return geometryShader;
    }

    public boolean isTessellationShader() {
        return tessellationShader;
    }

    public boolean isMultiDrawIndirect() {
        return multiDrawIndirect;
    }

    public boolean isDrawIndirectFirstInstance() {
        return drawIndirectFirstInstance;
    }

    public boolean isDepthClamp() {
        return depthClamp;
    }

    public boolean isDepthBiasClamp() {
        return depthBiasClamp;
    }

    public boolean isFillModeNonSolid() {
        return fillModeNonSolid;
    }

    public boolean isWideLines() {
        return wideLines;
    }

    public boolean isSamplerAnisotropy() {
        return samplerAnisotropy;
    }

    public boolean isTextureCompressionBC() {
        return textureCompressionBC;
    }

    public boolean isShaderFloat64() {
        return shaderFloat64;
    }

    public boolean isShaderInt64() {
        return shaderInt64;
    }

    public boolean isShaderInt16() {
        return shaderInt16;
    }

    public boolean isComputeSupported() {
        return computeSupported;
    }

    public boolean isRayTracingSupported() {
        return rayTracingSupported;
    }

    public boolean isMeshShaderSupported() {
        return meshShaderSupported;
    }

    public boolean isBindlessSupported() {
        return bindlessSupported;
    }

    public long getDeviceLocalMemoryMB() {
        return deviceLocalMemoryMB;
    }

    public long getHostVisibleMemoryMB() {
        return hostVisibleMemoryMB;
    }

    // =========================================================================
    // TO STRING
    // =========================================================================

    @Override
    public String toString() {
        if (!available) {
            return "VulkanCapabilities[Not Available]";
        }
        return String.format("VulkanCapabilities[%s, Device=%s, RT=%b, Mesh=%b, Compute=%b]",
                getVersionString(), deviceName, rayTracingSupported, meshShaderSupported, computeSupported);
    }

    // =========================================================================
    // BUILDER
    // =========================================================================

    public static class Builder {
        private boolean available = false;
        private int versionMajor = 0;
        private int versionMinor = 0;
        private int versionPatch = 0;
        private String deviceName = "Unknown";
        private String driverVersion = "Unknown";
        private int vendorId = 0;
        private int deviceId = 0;
        private DeviceType deviceType = DeviceType.OTHER;
        private List<String> extensions = new ArrayList<>();
        private List<String> layers = new ArrayList<>();

        private int maxImageDimension2D = 4096;
        private int maxImageDimension3D = 256;
        private int maxImageArrayLayers = 256;
        private int maxUniformBufferRange = 16384;
        private int maxStorageBufferRange = 134217728;
        private int maxPushConstantsSize = 128;
        private int maxMemoryAllocationCount = 4096;
        private int maxBoundDescriptorSets = 4;
        private int maxComputeSharedMemorySize = 32768;
        private int maxComputeWorkGroupInvocations = 1024;
        private int[] maxComputeWorkGroupCount = { 65535, 65535, 65535 };
        private int[] maxComputeWorkGroupSize = { 1024, 1024, 64 };

        private boolean geometryShader = false;
        private boolean tessellationShader = false;
        private boolean multiDrawIndirect = false;
        private boolean drawIndirectFirstInstance = false;
        private boolean depthClamp = false;
        private boolean depthBiasClamp = false;
        private boolean fillModeNonSolid = false;
        private boolean wideLines = false;
        private boolean samplerAnisotropy = true;
        private boolean textureCompressionBC = true;
        private boolean shaderFloat64 = false;
        private boolean shaderInt64 = false;
        private boolean shaderInt16 = false;

        private boolean computeSupported = true;
        private boolean rayTracingSupported = false;
        private boolean meshShaderSupported = false;
        private boolean bindlessSupported = false;

        private long deviceLocalMemoryMB = 0;
        private long hostVisibleMemoryMB = 0;

        public Builder setAvailable(boolean available) {
            this.available = available;
            return this;
        }

        public Builder setVersionMajor(int version) {
            this.versionMajor = version;
            return this;
        }

        public Builder setVersionMinor(int version) {
            this.versionMinor = version;
            return this;
        }

        public Builder setVersionPatch(int version) {
            this.versionPatch = version;
            return this;
        }

        public Builder setDeviceName(String name) {
            this.deviceName = name;
            return this;
        }

        public Builder setDriverVersion(String version) {
            this.driverVersion = version;
            return this;
        }

        public Builder setVendorId(int id) {
            this.vendorId = id;
            return this;
        }

        public Builder setDeviceId(int id) {
            this.deviceId = id;
            return this;
        }

        public Builder setDeviceType(DeviceType type) {
            this.deviceType = type;
            return this;
        }

        public Builder addExtension(String ext) {
            this.extensions.add(ext);
            return this;
        }

        public Builder addLayer(String layer) {
            this.layers.add(layer);
            return this;
        }

        public Builder setMaxImageDimension2D(int val) {
            this.maxImageDimension2D = val;
            return this;
        }

        public Builder setMaxImageDimension3D(int val) {
            this.maxImageDimension3D = val;
            return this;
        }

        public Builder setComputeSupported(boolean val) {
            this.computeSupported = val;
            return this;
        }

        public Builder setRayTracingSupported(boolean val) {
            this.rayTracingSupported = val;
            return this;
        }

        public Builder setMeshShaderSupported(boolean val) {
            this.meshShaderSupported = val;
            return this;
        }

        public Builder setBindlessSupported(boolean val) {
            this.bindlessSupported = val;
            return this;
        }

        public Builder setDeviceLocalMemoryMB(long val) {
            this.deviceLocalMemoryMB = val;
            return this;
        }

        public Builder setHostVisibleMemoryMB(long val) {
            this.hostVisibleMemoryMB = val;
            return this;
        }

        public VulkanCapabilities build() {
            return new VulkanCapabilities(this);
        }
    }
}
