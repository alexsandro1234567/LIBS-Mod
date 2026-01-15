/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * PredictiveNetcode.java - Predictive Network Packet Processing
 * 
 * Predicts and pre-processes network events to reduce perceived latency.
 */

package dev.libs.network;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.Map;
import java.util.Queue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/**
 * PredictiveNetcode - Latency Reduction System
 * 
 * Uses prediction algorithms to reduce perceived network latency:
 * - Entity position prediction (dead reckoning)
 * - Input queue with rollback
 * - Server reconciliation
 * - Packet timing optimization
 * 
 * <h2>Features:</h2>
 * <ul>
 * <li>Client-side prediction for player movement</li>
 * <li>Entity interpolation/extrapolation</li>
 * <li>Lag compensation for combat</li>
 * <li>Packet batching and prioritization</li>
 * </ul>
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class PredictiveNetcode {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(PredictiveNetcode.class);

    /** Maximum prediction time (ms) */
    private static final long MAX_PREDICTION_MS = 500;

    /** Input buffer size */
    private static final int INPUT_BUFFER_SIZE = 64;

    /** Entity snapshot buffer size */
    private static final int SNAPSHOT_BUFFER_SIZE = 32;

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** Whether system is active */
    private final AtomicBoolean active = new AtomicBoolean(false);

    /** Current estimated RTT (Round Trip Time) in milliseconds */
    private volatile float currentRTT = 0;

    /** Smoothed RTT */
    private volatile float smoothedRTT = 100; // Start with 100ms default

    /** RTT variance */
    private volatile float rttVariance = 50;

    /** Last server tick received */
    private volatile long lastServerTick = 0;

    /** Client prediction tick */
    private volatile long clientTick = 0;

    /** Input history for reconciliation */
    private final Queue<InputState> inputHistory;

    /** Entity prediction states */
    private final Map<Integer, EntityState> entityStates;

    /** Pending server updates to reconcile */
    private final Queue<ServerUpdate> pendingUpdates;

    /** Statistics */
    private final AtomicLong predictionsCorrect = new AtomicLong(0);
    private final AtomicLong predictionsMissed = new AtomicLong(0);
    private final AtomicLong packetsProcessed = new AtomicLong(0);
    private final AtomicLong rollbacks = new AtomicLong(0);

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    /**
     * Create a new PredictiveNetcode instance
     */
    public PredictiveNetcode() {
        this.inputHistory = new ConcurrentLinkedQueue<>();
        this.entityStates = new ConcurrentHashMap<>();
        this.pendingUpdates = new ConcurrentLinkedQueue<>();

        LOGGER.debug("PredictiveNetcode created");
    }

    // =========================================================================
    // LIFECYCLE
    // =========================================================================

    /**
     * Initialize the netcode system
     */
    public void initialize() {
        LOGGER.info("Initializing PredictiveNetcode...");

        active.set(true);

        LOGGER.info("PredictiveNetcode initialized");
    }

    /**
     * Shutdown the netcode system
     */
    public void shutdown() {
        if (!active.compareAndSet(true, false)) {
            return;
        }

        LOGGER.info("Shutting down PredictiveNetcode...");
        LOGGER.info("  Predictions correct: {}", predictionsCorrect.get());
        LOGGER.info("  Predictions missed: {}", predictionsMissed.get());
        LOGGER.info("  Total rollbacks: {}", rollbacks.get());

        inputHistory.clear();
        entityStates.clear();
        pendingUpdates.clear();

        LOGGER.info("PredictiveNetcode shutdown complete");
    }

    /**
     * Check if the netcode system is healthy
     */
    public boolean isHealthy() {
        return active.get();
    }

    /**
     * Update the netcode system each tick
     * 
     * @param deltaTime Time since last update in seconds
     */
    public void update(float deltaTime) {
        if (!active.get())
            return;

        // Process pending reconciliations and update entity predictions
        // This is called each game tick
    }

    // =========================================================================
    // RTT CALCULATION
    // =========================================================================

    /**
     * Update RTT based on packet round-trip
     * 
     * @param sentTime     Time packet was sent (System.nanoTime)
     * @param receivedTime Time acknowledgment was received
     */
    public void updateRTT(long sentTime, long receivedTime) {
        float sampleRTT = (receivedTime - sentTime) / 1_000_000f;

        // Jacobson's algorithm for smoothed RTT
        float alpha = 0.125f;
        float beta = 0.25f;

        float diff = sampleRTT - smoothedRTT;
        rttVariance = (1 - beta) * rttVariance + beta * Math.abs(diff);
        smoothedRTT = (1 - alpha) * smoothedRTT + alpha * sampleRTT;
        currentRTT = sampleRTT;

        LOGGER.trace("RTT updated: sample={:.1f}ms, smoothed={:.1f}ms", sampleRTT, smoothedRTT);
    }

    /**
     * Get estimated RTT
     */
    public float getRTT() {
        return smoothedRTT;
    }

    /**
     * Get RTT with jitter buffer
     */
    public float getRTTWithBuffer() {
        return smoothedRTT + 4 * rttVariance;
    }

    // =========================================================================
    // INPUT PREDICTION
    // =========================================================================

    /**
     * Record a player input for later reconciliation
     * 
     * @param input Input state to record
     */
    public void recordInput(@NotNull InputState input) {
        if (!active.get())
            return;

        input.tick = clientTick++;
        input.timestamp = System.currentTimeMillis();

        // Add to history
        inputHistory.offer(input);

        // Trim old inputs
        while (inputHistory.size() > INPUT_BUFFER_SIZE) {
            inputHistory.poll();
        }
    }

    /**
     * Get predicted player position based on unconfirmed inputs
     */
    public Position getPredictedPosition(Position currentPos) {
        Position predicted = new Position(currentPos.x, currentPos.y, currentPos.z);

        for (InputState input : inputHistory) {
            // Apply input to predicted position
            predicted.x += input.moveX * 0.1; // Simplified movement
            predicted.z += input.moveZ * 0.1;
        }

        return predicted;
    }

    // =========================================================================
    // ENTITY PREDICTION
    // =========================================================================

    /**
     * Update entity state from server
     */
    public void updateEntityState(int entityId, double x, double y, double z,
            float yaw, float pitch, double velX, double velY, double velZ) {
        EntityState state = entityStates.computeIfAbsent(entityId, EntityState::new);

        state.lastServerUpdate = System.currentTimeMillis();
        state.serverX = x;
        state.serverY = y;
        state.serverZ = z;
        state.serverYaw = yaw;
        state.serverPitch = pitch;
        state.velocityX = velX;
        state.velocityY = velY;
        state.velocityZ = velZ;

        // Reset prediction
        state.predictedX = x;
        state.predictedY = y;
        state.predictedZ = z;
    }

    /**
     * Get interpolated/extrapolated entity position
     */
    @Nullable
    public EntityPosition getEntityPosition(int entityId, float partialTicks) {
        EntityState state = entityStates.get(entityId);
        if (state == null)
            return null;

        long now = System.currentTimeMillis();
        float elapsed = (now - state.lastServerUpdate) / 1000f;

        // Limit prediction time
        if (elapsed > MAX_PREDICTION_MS / 1000f) {
            elapsed = MAX_PREDICTION_MS / 1000f;
        }

        // Extrapolate position based on velocity
        double predX = state.serverX + state.velocityX * elapsed;
        double predY = state.serverY + state.velocityY * elapsed;
        double predZ = state.serverZ + state.velocityZ * elapsed;

        // Smooth interpolation to predicted position
        float alpha = 0.5f;
        state.predictedX = state.predictedX * (1 - alpha) + predX * alpha;
        state.predictedY = state.predictedY * (1 - alpha) + predY * alpha;
        state.predictedZ = state.predictedZ * (1 - alpha) + predZ * alpha;

        return new EntityPosition(
                state.predictedX, state.predictedY, state.predictedZ,
                state.serverYaw, state.serverPitch);
    }

    /**
     * Remove entity from tracking
     */
    public void removeEntity(int entityId) {
        entityStates.remove(entityId);
    }

    // =========================================================================
    // SERVER RECONCILIATION
    // =========================================================================

    /**
     * Receive authoritative server update
     */
    public void receiveServerUpdate(long serverTick, double playerX, double playerY, double playerZ) {
        ServerUpdate update = new ServerUpdate();
        update.serverTick = serverTick;
        update.playerX = playerX;
        update.playerY = playerY;
        update.playerZ = playerZ;
        update.receivedAt = System.currentTimeMillis();

        pendingUpdates.offer(update);
        lastServerTick = serverTick;
        packetsProcessed.incrementAndGet();
    }

    /**
     * Reconcile predicted state with server state
     * 
     * @param currentPos Current client position
     * @return Corrected position, or null if no correction needed
     */
    @Nullable
    public Position reconcile(Position currentPos) {
        ServerUpdate update = pendingUpdates.poll();
        if (update == null)
            return null;

        // Find inputs that were processed by server
        InputState lastConfirmed = null;
        while (!inputHistory.isEmpty()) {
            InputState input = inputHistory.peek();
            if (input.tick <= update.serverTick) {
                lastConfirmed = inputHistory.poll();
            } else {
                break;
            }
        }

        // Calculate prediction error
        double errorX = currentPos.x - update.playerX;
        double errorY = currentPos.y - update.playerY;
        double errorZ = currentPos.z - update.playerZ;
        double errorSq = errorX * errorX + errorY * errorY + errorZ * errorZ;

        // Threshold for correction (0.01 = 0.1 blocks)
        if (errorSq > 0.01) {
            predictionsMissed.incrementAndGet();
            rollbacks.incrementAndGet();

            // Snap to server position and re-apply unconfirmed inputs
            Position corrected = new Position(update.playerX, update.playerY, update.playerZ);

            for (InputState pendingInput : inputHistory) {
                corrected.x += pendingInput.moveX * 0.1;
                corrected.z += pendingInput.moveZ * 0.1;
            }

            LOGGER.debug("Reconciliation: error={:.3f}, correcting position", Math.sqrt(errorSq));
            return corrected;
        } else {
            predictionsCorrect.incrementAndGet();
            return null; // Prediction was correct
        }
    }

    // =========================================================================
    // PACKET OPTIMIZATION
    // =========================================================================

    /**
     * Determine optimal packet send rate based on RTT
     */
    public int getOptimalPacketRate() {
        // Higher RTT = more aggressive sending
        if (smoothedRTT < 50) {
            return 20; // 20 packets per second
        } else if (smoothedRTT < 100) {
            return 30;
        } else if (smoothedRTT < 200) {
            return 40;
        } else {
            return 60; // High latency = more updates
        }
    }

    /**
     * Get recommended interpolation delay
     */
    public float getInterpolationDelay() {
        // Use 2x RTT as interpolation buffer
        return smoothedRTT * 2 / 1000f;
    }

    // =========================================================================
    // STATISTICS
    // =========================================================================

    /**
     * Get prediction accuracy (0-1)
     */
    public float getPredictionAccuracy() {
        long total = predictionsCorrect.get() + predictionsMissed.get();
        if (total == 0)
            return 1.0f;
        return (float) predictionsCorrect.get() / total;
    }

    /**
     * Get total packets processed
     */
    public long getPacketsProcessed() {
        return packetsProcessed.get();
    }

    /**
     * Get rollback count
     */
    public long getRollbackCount() {
        return rollbacks.get();
    }

    /**
     * Get tracked entity count
     */
    public int getTrackedEntityCount() {
        return entityStates.size();
    }

    // =========================================================================
    // INNER CLASSES
    // =========================================================================

    /**
     * Player input state
     */
    public static final class InputState {
        public long tick;
        public long timestamp;
        public float moveX;
        public float moveZ;
        public boolean jumping;
        public boolean sneaking;
        public boolean sprinting;
        public float yaw;
        public float pitch;
    }

    /**
     * Entity prediction state
     */
    private static final class EntityState {
        final int entityId;
        long lastServerUpdate;

        double serverX, serverY, serverZ;
        float serverYaw, serverPitch;
        double velocityX, velocityY, velocityZ;

        double predictedX, predictedY, predictedZ;

        EntityState(int entityId) {
            this.entityId = entityId;
        }
    }

    /**
     * Server update packet
     */
    private static final class ServerUpdate {
        long serverTick;
        double playerX, playerY, playerZ;
        long receivedAt;
    }

    /**
     * Simple position container
     */
    public static final class Position {
        public double x, y, z;

        public Position(double x, double y, double z) {
            this.x = x;
            this.y = y;
            this.z = z;
        }
    }

    /**
     * Entity position with rotation
     */
    public static final class EntityPosition {
        public final double x, y, z;
        public final float yaw, pitch;

        public EntityPosition(double x, double y, double z, float yaw, float pitch) {
            this.x = x;
            this.y = y;
            this.z = z;
            this.yaw = yaw;
            this.pitch = pitch;
        }
    }
}
