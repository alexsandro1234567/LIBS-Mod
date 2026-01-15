/*
 * Project Aether - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * AetherMod.java - Forge 1.8.9 Mod Entry Point
 */

package dev.aether.forge;

import dev.aether.AetherCore;
import dev.aether.AetherConfig;

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
 * Aether Mod - Forge 1.8.9 Entry Point
 * 
 * Main mod class for Forge loader on Minecraft 1.8.9.
 * Uses legacy Forge event system.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 */
@Mod(modid = AetherMod.MOD_ID, name = AetherMod.MOD_NAME, version = AetherMod.VERSION, acceptedMinecraftVersions = "[1.8.9]")
public class AetherMod {

    public static final String MOD_ID = "aether";
    public static final String MOD_NAME = "Project Aether";
    public static final String VERSION = "1.0.0-alpha";

    private static final Logger LOGGER = LogManager.getLogger(MOD_NAME);

    @Mod.Instance(MOD_ID)
    public static AetherMod instance;

    private AetherCore core;

    @EventHandler
    public void preInit(FMLPreInitializationEvent event) {
        LOGGER.info("╔════════════════════════════════════════════════════════════════╗");
        LOGGER.info("║                    PROJECT AETHER                               ║");
        LOGGER.info("║                  Forge 1.8.9 Edition                            ║");
        LOGGER.info("║         by Aiblox (Alexsandro Alves de Oliveira)                ║");
        LOGGER.info("╚════════════════════════════════════════════════════════════════╝");

        // Initialize core synchronously (1.8.9 Forge runs preInit on main thread)
        try {
            this.core = AetherCore.initialize().get(); // Blocks until initialized
            LOGGER.info("AetherCore initialized successfully");
        } catch (Exception e) {
            LOGGER.error("Failed to initialize AetherCore: " + e.getMessage(), e);
            return; // Cannot continue without core
        }

        // Pre-initialization
        try {
            core.preInit();
            LOGGER.info("Aether pre-initialization complete");
        } catch (Exception e) {
            LOGGER.error("Failed to pre-initialize Aether: " + e.getMessage(), e);
        }
    }

    @EventHandler
    public void init(FMLInitializationEvent event) {
        LOGGER.info("Aether initialization...");

        // Full initialization
        try {
            core.init();

            // Register event handler
            MinecraftForge.EVENT_BUS.register(this);

            LOGGER.info("Aether initialization complete");
        } catch (Exception e) {
            LOGGER.error("Failed to initialize Aether: " + e.getMessage(), e);
        }
    }

    @EventHandler
    public void postInit(FMLPostInitializationEvent event) {
        LOGGER.info("Aether post-initialization...");

        try {
            core.postInit();
            LOGGER.info("Aether post-initialization complete");
        } catch (Exception e) {
            LOGGER.error("Failed to post-initialize Aether: " + e.getMessage(), e);
        }
    }

    @SubscribeEvent
    public void onClientTick(TickEvent.ClientTickEvent event) {
        if (event.phase == TickEvent.Phase.START && core.isInitialized()) {
            core.clientTick();
        }
    }

    @SubscribeEvent
    public void onServerTick(TickEvent.ServerTickEvent event) {
        if (event.phase == TickEvent.Phase.START && core.isInitialized()) {
            core.serverTick();
        }
    }

    @SubscribeEvent
    public void onRenderTick(TickEvent.RenderTickEvent event) {
        if (event.phase == TickEvent.Phase.START && core.isInitialized()) {
            core.renderTick(event.renderTickTime);
        }
    }

    /**
     * Get core instance
     */
    public AetherCore getCore() {
        return core;
    }
}
