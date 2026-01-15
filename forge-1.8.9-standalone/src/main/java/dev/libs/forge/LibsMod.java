/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * LibsMod.java - Forge 1.8.9 Mod Entry Point
 */

package dev.libs.forge;

import dev.libs.LibsCore;
import dev.libs.LibsConfig;

import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.Mod.EventHandler;
import net.minecraftforge.fml.common.event.FMLInitializationEvent;
import net.minecraftforge.fml.common.event.FMLPreInitializationEvent;
import net.minecraftforge.fml.common.event.FMLPostInitializationEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/**
 * Libs Mod - Forge 1.8.9 Entry Point
 * 
 * Main mod class for Forge loader on Minecraft 1.8.9.
 * Uses legacy Forge event system.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 */
@Mod(modid = LibsMod.MOD_ID, name = LibsMod.MOD_NAME, version = LibsMod.VERSION, acceptedMinecraftVersions = "[1.8.9]")
public class LibsMod {

    public static final String MOD_ID = "libs";
    public static final String MOD_NAME = "LIBS";
    public static final String VERSION = "1.0.0-alpha";

    private static final Logger LOGGER = LogManager.getLogger(MOD_NAME);

    @Mod.Instance(MOD_ID)
    public static LibsMod instance;

    private LibsCore core;

    @EventHandler
    public void preInit(FMLPreInitializationEvent event) {
        LOGGER.info("╔═══════════════════════════════════════════════════════════════╗");
        LOGGER.info("║                          LIBS                                 ║");
        LOGGER.info("║                   Forge 1.8.9 Edition                         ║");
        LOGGER.info("║        by Aiblox (Alexsandro Alves de Oliveira)               ║");
        LOGGER.info("╚═══════════════════════════════════════════════════════════════╝");

        // Initialize core synchronously (1.8.9 Forge runs preInit on main thread)
        try {
            this.core = LibsCore.initialize().get(); // Blocks until initialized
            LOGGER.info("LibsCore initialized successfully");
        } catch (Exception e) {
            LOGGER.error("Failed to initialize LibsCore: " + e.getMessage(), e);
            return; // Cannot continue without core
        }

        // Pre-initialization
        try {
            core.preInit();
            LOGGER.info("LIBS pre-initialization complete");
        } catch (Exception e) {
            LOGGER.error("Failed to pre-initialize LIBS: " + e.getMessage(), e);
        }
    }

    @EventHandler
    public void init(FMLInitializationEvent event) {
        LOGGER.info("LIBS initialization...");

        // Full initialization
        try {
            core.init();

            // Register event handler
            MinecraftForge.EVENT_BUS.register(this);

            LOGGER.info("LIBS initialization complete");
        } catch (Exception e) {
            LOGGER.error("Failed to initialize LIBS: " + e.getMessage(), e);
        }
    }

    @EventHandler
    public void postInit(FMLPostInitializationEvent event) {
        LOGGER.info("LIBS post-initialization...");

        // Post-init is handled internally by LibsCore
        if (core != null && core.isInitialized()) {
            LOGGER.info("LIBS post-initialization complete");
        }
    }

    @SubscribeEvent
    public void onClientTick(TickEvent.ClientTickEvent event) {
        if (event.phase == TickEvent.Phase.START && core != null && core.isInitialized()) {
            core.clientTick();
        }
    }

    @SubscribeEvent
    public void onServerTick(TickEvent.ServerTickEvent event) {
        if (event.phase == TickEvent.Phase.START && core != null && core.isInitialized()) {
            core.serverTick();
        }
    }

    @SubscribeEvent
    public void onRenderTick(TickEvent.RenderTickEvent event) {
        if (event.phase == TickEvent.Phase.START && core != null && core.isInitialized()) {
            core.onPreRender(event.renderTickTime);
        }
    }

    /**
     * Get core instance
     */
    public LibsCore getCore() {
        return core;
    }
}
