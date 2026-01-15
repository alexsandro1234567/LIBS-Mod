/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * NativeExtractionResult.java - Extraction Result Data Class
 */

package dev.libs.loader;

import org.jetbrains.annotations.NotNull;

import java.nio.file.Path;

/**
 * NativeExtractionResult - Result of native library extraction
 * 
 * Contains information about the extracted library including:
 * - Path to the extracted file
 * - File size
 * - SHA-256 hash
 * - Verification status
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class NativeExtractionResult {

    private final Path path;
    private final long sizeBytes;
    private final String sha256Hash;
    private final boolean hashValid;
    private final boolean wasAlreadyExtracted;

    /**
     * Create a new extraction result
     * 
     * @param path                Path to extracted library
     * @param sizeBytes           File size in bytes
     * @param sha256Hash          SHA-256 hash of the file
     * @param hashValid           Whether the hash matched expected value
     * @param wasAlreadyExtracted Whether the file was already extracted (cached)
     */
    public NativeExtractionResult(
            @NotNull Path path,
            long sizeBytes,
            @NotNull String sha256Hash,
            boolean hashValid,
            boolean wasAlreadyExtracted) {
        this.path = path;
        this.sizeBytes = sizeBytes;
        this.sha256Hash = sha256Hash;
        this.hashValid = hashValid;
        this.wasAlreadyExtracted = wasAlreadyExtracted;
    }

    /**
     * Get the path to the extracted library
     */
    @NotNull
    public Path getPath() {
        return path;
    }

    /**
     * Get the file size in bytes
     */
    public long getSizeBytes() {
        return sizeBytes;
    }

    /**
     * Get the file size in kilobytes
     */
    public long getSizeKB() {
        return sizeBytes / 1024;
    }

    /**
     * Get the file size in megabytes
     */
    public double getSizeMB() {
        return sizeBytes / (1024.0 * 1024.0);
    }

    /**
     * Get the SHA-256 hash of the extracted file
     */
    @NotNull
    public String getSha256Hash() {
        return sha256Hash;
    }

    /**
     * Check if the hash matched the expected value
     */
    public boolean isHashValid() {
        return hashValid;
    }

    /**
     * Check if the file was already extracted (cached)
     */
    public boolean wasAlreadyExtracted() {
        return wasAlreadyExtracted;
    }

    /**
     * Check if extraction was successful (file exists and hash valid)
     */
    public boolean isSuccess() {
        return path != null && (hashValid || sha256Hash != null);
    }

    @Override
    public String toString() {
        return String.format(
                "NativeExtractionResult[path=%s, size=%d bytes, hash=%s, valid=%b, cached=%b]",
                path, sizeBytes, sha256Hash.substring(0, 16) + "...", hashValid, wasAlreadyExtracted);
    }
}
