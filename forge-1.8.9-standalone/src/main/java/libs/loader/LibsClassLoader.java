/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * LibsClassLoader.java - Custom ClassLoader for Native Library Integration
 * 
 * Provides special class loading capabilities for native library path injection.
 */

package dev.libs.loader;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.io.File;
import java.io.IOException;
import java.lang.reflect.Field;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import java.util.Vector;

/**
 * LibsClassLoader - Custom ClassLoader for Libs
 * 
 * Provides special capabilities for the Libs mod:
 * - Native library path injection
 * - Class isolation where needed
 * - Dynamic library path updates
 * 
 * <h2>Usage:</h2>
 * 
 * <pre>
 * LibsClassLoader loader = LibsClassLoader.getInstance();
 * loader.addNativeLibraryPath(nativesDir);
 * </pre>
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class LibsClassLoader extends URLClassLoader {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(LibsClassLoader.class);

    // =========================================================================
    // STATIC FIELDS
    // =========================================================================

    /** Singleton instance */
    private static volatile LibsClassLoader instance;

    /** Paths that have been added to the native library path */
    private static final Set<Path> addedNativePaths = new HashSet<>();

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new LibsClassLoader
     * 
     * @param urls   URLs to add to the classpath
     * @param parent Parent classloader
     */
    public LibsClassLoader(URL[] urls, ClassLoader parent) {
        super(urls, parent);
    }

    /**
     * Get or create the singleton instance
     */
    public static LibsClassLoader getInstance() {
        if (instance == null) {
            synchronized (LibsClassLoader.class) {
                if (instance == null) {
                    instance = new LibsClassLoader(
                            new URL[0],
                            LibsClassLoader.class.getClassLoader());
                }
            }
        }
        return instance;
    }

    // =========================================================================
    // NATIVE LIBRARY PATH MANIPULATION
    // =========================================================================

    /**
     * Add a directory to the native library search path.
     * This modifies java.library.path at runtime.
     * 
     * @param path Directory to add
     * @throws IOException if path cannot be added
     */
    public static void addNativeLibraryPath(@NotNull Path path) throws IOException {
        if (addedNativePaths.contains(path)) {
            LOGGER.debug("Native path already added: {}", path);
            return;
        }

        LOGGER.info("Adding native library path: {}", path);

        try {
            // Method 1: Try to add to sys_paths field
            addToSysPaths(path.toAbsolutePath().toString());

            // Method 2: Also update java.library.path property
            String currentPath = System.getProperty("java.library.path", "");
            String newPath = path.toAbsolutePath().toString();

            if (!currentPath.contains(newPath)) {
                String separator = File.pathSeparator;
                String updatedPath = currentPath.isEmpty()
                        ? newPath
                        : currentPath + separator + newPath;
                System.setProperty("java.library.path", updatedPath);
            }

            addedNativePaths.add(path);
            LOGGER.debug("Native library path added successfully");

        } catch (Exception e) {
            LOGGER.warn("Could not dynamically add native path. Library loading may still work.", e);
            // Still mark as added since we updated the property
            addedNativePaths.add(path);
        }
    }

    /**
     * Add to the internal sys_paths array using reflection
     */
    private static void addToSysPaths(String path) throws Exception {
        try {
            // Try to set sys_paths to null to force reload
            Field sysPathsField = ClassLoader.class.getDeclaredField("sys_paths");
            sysPathsField.setAccessible(true);
            sysPathsField.set(null, null);
        } catch (NoSuchFieldException e) {
            // Java 9+ doesn't have sys_paths
            LOGGER.debug("sys_paths not available (Java 9+)");
        } catch (IllegalAccessException e) {
            // Module system blocking access
            LOGGER.debug("Cannot access sys_paths: {}", e.getMessage());
        }

        // Try to add via usr_paths
        try {
            Field usrPathsField = ClassLoader.class.getDeclaredField("usr_paths");
            usrPathsField.setAccessible(true);
            String[] paths = (String[]) usrPathsField.get(null);

            // Check if already present
            for (String p : paths) {
                if (p.equals(path)) {
                    return;
                }
            }

            // Add new path
            String[] newPaths = Arrays.copyOf(paths, paths.length + 1);
            newPaths[paths.length] = path;
            usrPathsField.set(null, newPaths);

        } catch (NoSuchFieldException e) {
            LOGGER.debug("usr_paths not available");
        } catch (IllegalAccessException e) {
            LOGGER.debug("Cannot access usr_paths: {}", e.getMessage());
        }
    }

    /**
     * Get the current native library path
     */
    public static String getNativeLibraryPath() {
        return System.getProperty("java.library.path", "");
    }

    /**
     * Check if a path has been added
     */
    public static boolean hasNativePath(Path path) {
        return addedNativePaths.contains(path);
    }

    // =========================================================================
    // URL MANAGEMENT
    // =========================================================================

    /**
     * Add a URL to the classpath
     * 
     * @param url URL to add
     */
    @Override
    public void addURL(URL url) {
        super.addURL(url);
        LOGGER.debug("Added URL to classpath: {}", url);
    }

    /**
     * Add a path to the classpath
     * 
     * @param path Path to add (file or directory)
     * @throws IOException if path cannot be converted to URL
     */
    public void addPath(@NotNull Path path) throws IOException {
        addURL(path.toUri().toURL());
    }

    // =========================================================================
    // CLASS LOADING
    // =========================================================================

    @Override
    protected Class<?> findClass(String name) throws ClassNotFoundException {
        // Check if this is an Libs class that needs special handling
        if (name.startsWith("dev.libs.")) {
            LOGGER.trace("Loading Libs class: {}", name);
        }

        return super.findClass(name);
    }

    @Override
    public Class<?> loadClass(String name) throws ClassNotFoundException {
        return loadClass(name, false);
    }

    @Override
    protected Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
        synchronized (getClassLoadingLock(name)) {
            // Check if already loaded
            Class<?> c = findLoadedClass(name);

            if (c == null) {
                // For Libs classes, try to load from this loader first
                if (name.startsWith("dev.libs.")) {
                    try {
                        c = findClass(name);
                    } catch (ClassNotFoundException e) {
                        // Fall through to parent
                    }
                }

                // If not found, delegate to parent
                if (c == null) {
                    c = getParent().loadClass(name);
                }
            }

            if (resolve) {
                resolveClass(c);
            }

            return c;
        }
    }

    // =========================================================================
    // UTILITY
    // =========================================================================

    /**
     * Print diagnostic information about the classloader
     */
    public void printDiagnostics() {
        LOGGER.info("=== LibsClassLoader Diagnostics ===");
        LOGGER.info("URLs: {}", Arrays.toString(getURLs()));
        LOGGER.info("Parent: {}", getParent());
        LOGGER.info("Native paths added: {}", addedNativePaths);
        LOGGER.info("java.library.path: {}", getNativeLibraryPath());
        LOGGER.info("=====================================");
    }

    @Override
    public void close() throws IOException {
        super.close();
        instance = null;
        addedNativePaths.clear();
        LOGGER.debug("LibsClassLoader closed");
    }
}
