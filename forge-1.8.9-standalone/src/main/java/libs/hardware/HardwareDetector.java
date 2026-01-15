/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * HardwareDetector.java - System Hardware Detection
 * 
 * Detects CPU, GPU, and memory capabilities for optimal configuration.
 * Generates hardware hash for native library selection.
 */

package dev.libs.hardware;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.lang.management.ManagementFactory;
import java.lang.management.OperatingSystemMXBean;
import java.lang.management.RuntimeMXBean;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * HardwareDetector - System Capability Detection
 * 
 * Detects hardware capabilities including:
 * - CPU model, cores, features (SSE, AVX, etc.)
 * - GPU model and Vulkan support
 * - System and available memory
 * - Operating system information
 * 
 * This information is used to:
 * - Select optimal native library variant
 * - Configure performance settings automatically
 * - Generate hardware fingerprint for optimization caching
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class HardwareDetector {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(HardwareDetector.class);

    /** OS Type enumeration */
    public enum OSType {
        WINDOWS, LINUX, MACOS, UNKNOWN
    }

    /** Architecture enumeration */
    public enum Architecture {
        X86_64, X86, ARM64, ARM32, UNKNOWN
    }

    // =========================================================================
    // FIELDS
    // =========================================================================

    private final OSType osType;
    private final Architecture architecture;

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    public HardwareDetector() {
        this.osType = detectOSType();
        this.architecture = detectArchitecture();

        LOGGER.debug("HardwareDetector initialized for {} {}", osType, architecture);
    }

    // =========================================================================
    // DETECTION
    // =========================================================================

    /**
     * Detect all hardware capabilities and return a profile
     * 
     * @return Hardware profile with all detected information
     */
    @NotNull
    public HardwareProfile detect() {
        LOGGER.info("Detecting hardware capabilities...");

        HardwareProfile.Builder builder = new HardwareProfile.Builder();

        // OS Information
        builder.setOsType(osType);
        builder.setOsName(System.getProperty("os.name", "Unknown"));
        builder.setOsVersion(System.getProperty("os.version", "Unknown"));
        builder.setArchitecture(architecture);

        // CPU Information
        detectCPU(builder);

        // GPU Information
        detectGPU(builder);

        // Memory Information
        detectMemory(builder);

        // Java Information
        detectJava(builder);

        // Generate hardware hash
        String hash = generateHardwareHash(builder);
        builder.setHardwareHash(hash);

        HardwareProfile profile = builder.build();

        LOGGER.info("Hardware detection complete:");
        LOGGER.info("  OS: {} {} ({})", profile.getOsName(), profile.getOsVersion(), architecture);
        LOGGER.info("  CPU: {} ({} cores)", profile.getCpuName(), profile.getCpuCores());
        LOGGER.info("  GPU: {}", profile.getGpuName());
        LOGGER.info("  Memory: {} MB total, {} MB available",
                profile.getTotalMemoryMB(), profile.getAvailableMemoryMB());
        LOGGER.info("  Hardware Hash: {}", hash);

        return profile;
    }

    // =========================================================================
    // OS DETECTION
    // =========================================================================

    private OSType detectOSType() {
        String osName = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);

        if (osName.contains("windows")) {
            return OSType.WINDOWS;
        } else if (osName.contains("linux")) {
            return OSType.LINUX;
        } else if (osName.contains("mac") || osName.contains("darwin")) {
            return OSType.MACOS;
        }

        return OSType.UNKNOWN;
    }

    private Architecture detectArchitecture() {
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);

        if (arch.contains("amd64") || arch.contains("x86_64") || arch.contains("x64")) {
            return Architecture.X86_64;
        } else if (arch.contains("x86") || arch.contains("i386") || arch.contains("i686")) {
            return Architecture.X86;
        } else if (arch.contains("aarch64") || arch.contains("arm64")) {
            return Architecture.ARM64;
        } else if (arch.contains("arm")) {
            return Architecture.ARM32;
        }

        return Architecture.UNKNOWN;
    }

    // =========================================================================
    // CPU DETECTION
    // =========================================================================

    private void detectCPU(HardwareProfile.Builder builder) {
        int cores = Runtime.getRuntime().availableProcessors();
        builder.setCpuCores(cores);

        String cpuName = "Unknown CPU";
        List<String> features = new ArrayList<>();

        switch (osType) {
            case WINDOWS:
                cpuName = detectCPUWindows();
                features = detectCPUFeaturesWindows();
                break;
            case LINUX:
                cpuName = detectCPULinux();
                features = detectCPUFeaturesLinux();
                break;
            case MACOS:
                cpuName = detectCPUMacOS();
                features = detectCPUFeaturesMacOS();
                break;
        }

        builder.setCpuName(cpuName);
        builder.setCpuFeatures(features);

        // Detect SIMD capabilities
        builder.setSseSupported(features.contains("sse") || features.contains("sse2"));
        builder.setAvxSupported(features.contains("avx"));
        builder.setAvx2Supported(features.contains("avx2"));
        builder.setAvx512Supported(features.contains("avx512f") || features.contains("avx512"));
    }

    private String detectCPUWindows() {
        try {
            ProcessBuilder pb = new ProcessBuilder("wmic", "cpu", "get", "name");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    line = line.trim();
                    if (!line.isEmpty() && !line.equalsIgnoreCase("Name")) {
                        return line;
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect CPU name via WMIC: {}", e.getMessage());
        }

        // Fallback to environment
        String processor = System.getenv("PROCESSOR_IDENTIFIER");
        if (processor != null && !processor.isEmpty()) {
            return processor;
        }

        return "Unknown CPU";
    }

    private String detectCPULinux() {
        try {
            ProcessBuilder pb = new ProcessBuilder("cat", "/proc/cpuinfo");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    if (line.startsWith("model name")) {
                        int colonIdx = line.indexOf(':');
                        if (colonIdx > 0) {
                            return line.substring(colonIdx + 1).trim();
                        }
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect CPU name from /proc/cpuinfo: {}", e.getMessage());
        }

        return "Unknown CPU";
    }

    private String detectCPUMacOS() {
        try {
            ProcessBuilder pb = new ProcessBuilder("sysctl", "-n", "machdep.cpu.brand_string");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line = reader.readLine();
                if (line != null && !line.isEmpty()) {
                    return line.trim();
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect CPU name via sysctl: {}", e.getMessage());
        }

        return "Unknown CPU";
    }

    private List<String> detectCPUFeaturesWindows() {
        List<String> features = new ArrayList<>();

        // On Windows, we assume modern features for x86_64
        if (architecture == Architecture.X86_64) {
            features.add("sse");
            features.add("sse2");
            features.add("sse3");
            features.add("ssse3");
            features.add("sse4_1");
            features.add("sse4_2");

            // Check for AVX support via environment variable hint
            String processor = System.getenv("PROCESSOR_IDENTIFIER");
            if (processor != null) {
                if (processor.contains("Intel") || processor.contains("AMD")) {
                    // Most modern Intel/AMD CPUs support AVX
                    features.add("avx");
                    features.add("avx2");
                }
            }
        }

        return features;
    }

    private List<String> detectCPUFeaturesLinux() {
        List<String> features = new ArrayList<>();

        try {
            ProcessBuilder pb = new ProcessBuilder("cat", "/proc/cpuinfo");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    if (line.startsWith("flags")) {
                        int colonIdx = line.indexOf(':');
                        if (colonIdx > 0) {
                            String[] flags = line.substring(colonIdx + 1).trim().split("\\s+");
                            for (String flag : flags) {
                                features.add(flag.toLowerCase());
                            }
                        }
                        break;
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect CPU features from /proc/cpuinfo: {}", e.getMessage());
        }

        return features;
    }

    private List<String> detectCPUFeaturesMacOS() {
        List<String> features = new ArrayList<>();

        try {
            ProcessBuilder pb = new ProcessBuilder("sysctl", "-n", "machdep.cpu.features");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line = reader.readLine();
                if (line != null) {
                    String[] flags = line.trim().split("\\s+");
                    for (String flag : flags) {
                        features.add(flag.toLowerCase());
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect CPU features via sysctl: {}", e.getMessage());
        }

        // Apple Silicon detection
        if (architecture == Architecture.ARM64) {
            features.add("neon");
            features.add("fp16");
        }

        return features;
    }

    // =========================================================================
    // GPU DETECTION
    // =========================================================================

    private void detectGPU(HardwareProfile.Builder builder) {
        String gpuName = "Unknown GPU";
        String gpuVendor = "Unknown";
        long gpuMemoryMB = 0;

        switch (osType) {
            case WINDOWS:
                Map<String, Object> gpuInfo = detectGPUWindows();
                gpuName = (String) gpuInfo.getOrDefault("name", "Unknown GPU");
                gpuVendor = (String) gpuInfo.getOrDefault("vendor", "Unknown");
                gpuMemoryMB = (Long) gpuInfo.getOrDefault("memoryMB", 0L);
                break;
            case LINUX:
                gpuInfo = detectGPULinux();
                gpuName = (String) gpuInfo.getOrDefault("name", "Unknown GPU");
                gpuVendor = (String) gpuInfo.getOrDefault("vendor", "Unknown");
                gpuMemoryMB = (Long) gpuInfo.getOrDefault("memoryMB", 0L);
                break;
            case MACOS:
                gpuInfo = detectGPUMacOS();
                gpuName = (String) gpuInfo.getOrDefault("name", "Unknown GPU");
                gpuVendor = (String) gpuInfo.getOrDefault("vendor", "Unknown");
                gpuMemoryMB = (Long) gpuInfo.getOrDefault("memoryMB", 0L);
                break;
        }

        builder.setGpuName(gpuName);
        builder.setGpuVendor(gpuVendor);
        builder.setGpuMemoryMB(gpuMemoryMB);

        // Detect GPU vendor type
        String gpuLower = gpuName.toLowerCase();
        if (gpuLower.contains("nvidia") || gpuLower.contains("geforce") || gpuLower.contains("rtx")
                || gpuLower.contains("gtx")) {
            builder.setGpuType(HardwareProfile.GPUType.NVIDIA);
        } else if (gpuLower.contains("amd") || gpuLower.contains("radeon")) {
            builder.setGpuType(HardwareProfile.GPUType.AMD);
        } else if (gpuLower.contains("intel")) {
            builder.setGpuType(HardwareProfile.GPUType.INTEL);
        } else if (gpuLower.contains("apple") || gpuLower.contains("m1") || gpuLower.contains("m2")
                || gpuLower.contains("m3")) {
            builder.setGpuType(HardwareProfile.GPUType.APPLE);
        } else {
            builder.setGpuType(HardwareProfile.GPUType.UNKNOWN);
        }
    }

    private Map<String, Object> detectGPUWindows() {
        Map<String, Object> info = new HashMap<>();
        info.put("name", "Unknown GPU");
        info.put("vendor", "Unknown");
        info.put("memoryMB", 0L);

        try {
            ProcessBuilder pb = new ProcessBuilder("wmic", "path", "win32_VideoController",
                    "get", "name,adapterram,driverversion");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                boolean headerPassed = false;
                while ((line = reader.readLine()) != null) {
                    line = line.trim();
                    if (line.isEmpty())
                        continue;

                    if (!headerPassed) {
                        headerPassed = true;
                        continue;
                    }

                    // Parse the output - format varies
                    String[] parts = line.split("\\s{2,}");
                    if (parts.length >= 1) {
                        // Try to extract memory size
                        Pattern memPattern = Pattern.compile("(\\d+)");
                        for (String part : parts) {
                            Matcher m = memPattern.matcher(part);
                            if (m.find()) {
                                try {
                                    long bytes = Long.parseLong(m.group(1));
                                    if (bytes > 100000000) { // Likely bytes
                                        info.put("memoryMB", bytes / (1024 * 1024));
                                    }
                                } catch (NumberFormatException ignored) {
                                }
                            }
                        }

                        // Find GPU name (usually longest part)
                        for (String part : parts) {
                            if (part.length() > 10 && !part.matches("\\d+(\\.\\d+)*")) {
                                info.put("name", part);
                                break;
                            }
                        }
                    }
                    break; // Only first GPU
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect GPU via WMIC: {}", e.getMessage());
        }

        return info;
    }

    private Map<String, Object> detectGPULinux() {
        Map<String, Object> info = new HashMap<>();
        info.put("name", "Unknown GPU");
        info.put("vendor", "Unknown");
        info.put("memoryMB", 0L);

        // Try lspci
        try {
            ProcessBuilder pb = new ProcessBuilder("lspci", "-v");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                boolean inVGA = false;
                while ((line = reader.readLine()) != null) {
                    if (line.contains("VGA compatible") || line.contains("3D controller")) {
                        inVGA = true;
                        // Extract GPU name
                        int colonIdx = line.indexOf(':');
                        if (colonIdx > 0) {
                            String gpuPart = line.substring(colonIdx + 1).trim();
                            // Remove device ID if present
                            int bracketIdx = gpuPart.indexOf('[');
                            if (bracketIdx > 0) {
                                int endBracket = gpuPart.indexOf(']', bracketIdx);
                                if (endBracket > bracketIdx) {
                                    gpuPart = gpuPart.substring(bracketIdx + 1, endBracket);
                                }
                            }
                            info.put("name", gpuPart.trim());
                        }
                    } else if (inVGA && line.contains("Memory")) {
                        // Try to extract memory size
                        Pattern memPattern = Pattern.compile("(\\d+)\\s*(M|G)");
                        Matcher m = memPattern.matcher(line);
                        if (m.find()) {
                            long size = Long.parseLong(m.group(1));
                            if (m.group(2).equals("G")) {
                                size *= 1024;
                            }
                            info.put("memoryMB", size);
                        }
                    } else if (inVGA && line.isEmpty()) {
                        break;
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect GPU via lspci: {}", e.getMessage());
        }

        return info;
    }

    private Map<String, Object> detectGPUMacOS() {
        Map<String, Object> info = new HashMap<>();
        info.put("name", "Unknown GPU");
        info.put("vendor", "Apple");
        info.put("memoryMB", 0L);

        try {
            ProcessBuilder pb = new ProcessBuilder("system_profiler", "SPDisplaysDataType");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    line = line.trim();

                    if (line.startsWith("Chipset Model:")) {
                        info.put("name", line.substring("Chipset Model:".length()).trim());
                    } else if (line.startsWith("VRAM")) {
                        Pattern memPattern = Pattern.compile("(\\d+)\\s*(MB|GB)");
                        Matcher m = memPattern.matcher(line);
                        if (m.find()) {
                            long size = Long.parseLong(m.group(1));
                            if (m.group(2).equals("GB")) {
                                size *= 1024;
                            }
                            info.put("memoryMB", size);
                        }
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to detect GPU via system_profiler: {}", e.getMessage());
        }

        return info;
    }

    // =========================================================================
    // MEMORY DETECTION
    // =========================================================================

    private void detectMemory(HardwareProfile.Builder builder) {
        Runtime runtime = Runtime.getRuntime();

        // Java heap memory
        long maxHeapMB = runtime.maxMemory() / (1024 * 1024);
        long totalJavaHeapMB = runtime.totalMemory() / (1024 * 1024);
        long freeJavaHeapMB = runtime.freeMemory() / (1024 * 1024);

        builder.setMaxJavaHeapMB(maxHeapMB);

        // System memory
        long totalSystemMemoryMB = 0;
        long availableSystemMemoryMB = 0;

        try {
            OperatingSystemMXBean osBean = ManagementFactory.getOperatingSystemMXBean();

            // Try to use com.sun.management extension
            if (osBean instanceof com.sun.management.OperatingSystemMXBean) {
                com.sun.management.OperatingSystemMXBean sunBean = (com.sun.management.OperatingSystemMXBean) osBean;

                totalSystemMemoryMB = sunBean.getTotalPhysicalMemorySize() / (1024 * 1024);
                availableSystemMemoryMB = sunBean.getFreePhysicalMemorySize() / (1024 * 1024);
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to get memory info from MXBean: {}", e.getMessage());
        }

        // Fallback: Read from OS
        if (totalSystemMemoryMB == 0) {
            switch (osType) {
                case WINDOWS:
                    totalSystemMemoryMB = getWindowsMemory();
                    break;
                case LINUX:
                    totalSystemMemoryMB = getLinuxMemory();
                    break;
                case MACOS:
                    totalSystemMemoryMB = getMacOSMemory();
                    break;
            }
        }

        builder.setTotalMemoryMB(totalSystemMemoryMB);
        builder.setAvailableMemoryMB(availableSystemMemoryMB > 0 ? availableSystemMemoryMB : totalSystemMemoryMB / 2);
    }

    private long getWindowsMemory() {
        try {
            ProcessBuilder pb = new ProcessBuilder("wmic", "OS", "get", "TotalVisibleMemorySize");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    line = line.trim();
                    if (!line.isEmpty() && !line.contains("TotalVisibleMemorySize")) {
                        try {
                            return Long.parseLong(line) / 1024; // KB to MB
                        } catch (NumberFormatException ignored) {
                        }
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to get Windows memory: {}", e.getMessage());
        }
        return 8192; // Default 8 GB
    }

    private long getLinuxMemory() {
        try {
            ProcessBuilder pb = new ProcessBuilder("cat", "/proc/meminfo");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    if (line.startsWith("MemTotal:")) {
                        Pattern pattern = Pattern.compile("(\\d+)");
                        Matcher m = pattern.matcher(line);
                        if (m.find()) {
                            return Long.parseLong(m.group(1)) / 1024; // KB to MB
                        }
                    }
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to get Linux memory: {}", e.getMessage());
        }
        return 8192;
    }

    private long getMacOSMemory() {
        try {
            ProcessBuilder pb = new ProcessBuilder("sysctl", "-n", "hw.memsize");
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream()))) {
                String line = reader.readLine();
                if (line != null) {
                    return Long.parseLong(line.trim()) / (1024 * 1024); // Bytes to MB
                }
            }
        } catch (Exception e) {
            LOGGER.debug("Failed to get macOS memory: {}", e.getMessage());
        }
        return 8192;
    }

    // =========================================================================
    // JAVA DETECTION
    // =========================================================================

    private void detectJava(HardwareProfile.Builder builder) {
        builder.setJavaVersion(System.getProperty("java.version", "Unknown"));
        builder.setJavaVendor(System.getProperty("java.vendor", "Unknown"));
        builder.setJavaHome(System.getProperty("java.home", ""));

        RuntimeMXBean runtimeBean = ManagementFactory.getRuntimeMXBean();
        builder.setJvmName(runtimeBean.getVmName());
        builder.setJvmVersion(runtimeBean.getVmVersion());
    }

    // =========================================================================
    // HASH GENERATION
    // =========================================================================

    /**
     * Generate a unique hardware hash for this system
     */
    private String generateHardwareHash(HardwareProfile.Builder builder) {
        try {
            StringBuilder sb = new StringBuilder();
            sb.append(osType.name());
            sb.append(architecture.name());
            sb.append(builder.getCpuName());
            sb.append(builder.getCpuCores());
            sb.append(builder.getGpuName());
            sb.append(builder.getTotalMemoryMB());

            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest(sb.toString().getBytes(StandardCharsets.UTF_8));

            // Convert to hex string (first 16 chars)
            StringBuilder hexString = new StringBuilder();
            for (int i = 0; i < 8; i++) {
                String hex = Integer.toHexString(0xff & hash[i]);
                if (hex.length() == 1) {
                    hexString.append('0');
                }
                hexString.append(hex);
            }

            return hexString.toString();

        } catch (Exception e) {
            LOGGER.warn("Failed to generate hardware hash: {}", e.getMessage());
            return "00000000";
        }
    }

    // =========================================================================
    // GETTERS
    // =========================================================================

    public OSType getOSType() {
        return osType;
    }

    public Architecture getArchitecture() {
        return architecture;
    }
}
