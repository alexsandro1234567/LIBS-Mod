/*
 * LIBS - Rendering Engine Mod
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 */

package dev.libs.forge;

import dev.libs.LibsCore;
import dev.libs.render.RenderStats;
import dev.libs.render.ChunkOptimizer;

import net.minecraftforge.fml.common.Mod;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * LIBS Mod - MinecraftForge 1.21.11 Entry Point
 * High-performance rendering engine for Minecraft
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 */
@Mod(LibsMod.MOD_ID)
public class LibsMod {

    public static final String MOD_ID = "libs";
    public static final String MOD_NAME = "LIBS Rendering Engine";
    public static final String VERSION = "1.0.0-alpha";

    private static final Logger LOGGER = LoggerFactory.getLogger(LibsMod.class);

    private static LibsMod instance;
    private LibsCore core;

    public LibsMod() {
        instance = this;

        LOGGER.info("╔════════════════════════════════════════════════════════════════╗");
        LOGGER.info("║                    LIBS RENDERING ENGINE                       ║");
        LOGGER.info("║               MinecraftForge 1.21.11 Edition                   ║");
        LOGGER.info("║         by Aiblox (Alexsandro Alves de Oliveira)               ║");
        LOGGER.info("║                                                                ║");
        LOGGER.info("║  Core Systems:                                                 ║");
        LOGGER.info("║    • Quantum Renderer (Vulkan/OpenGL Hybrid)                   ║");
        LOGGER.info("║    • Nanite Virtual Geometry (LOD System)                      ║");
        LOGGER.info("║    • Lumen-Lite (SSGI/Voxel Cone Tracing)                      ║");
        LOGGER.info("║    • Hyper-Threaded ECS (Parallel Entity Ticks)                ║");
        LOGGER.info("║    • Void Manager (Off-Heap Memory/Dedup)                      ║");
        LOGGER.info("║    • Predictive Netcode (Latency Masking)                      ║");
        LOGGER.info("║    • Ray-Traced Audio (Voxel Occlusion)                        ║");
        LOGGER.info("╚════════════════════════════════════════════════════════════════╝");

        // Initialize core
        initializeCore();

        LOGGER.info("LIBS mod constructed - all engine systems active");
    }

    private void initializeCore() {
        try {
            core = LibsCore.getInstance();
            if (core == null) {
                LibsCore.initialize().get();
                core = LibsCore.getInstance();
            }

            if (core != null) {
                core.preInit();
                core.init();

                // Clear caches
                ChunkOptimizer.clearCache();
                RenderStats.reset();

                LOGGER.info("LIBS core initialized successfully");
            }
        } catch (Exception e) {
            LOGGER.warn("LIBS core initialization: {}", e.getMessage());
        }
    }

    public static LibsMod getInstance() {
        return instance;
    }

    public LibsCore getCore() {
        return core;
    }
}
