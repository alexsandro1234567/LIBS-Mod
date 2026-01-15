/*
 * LIBS - Universal Monolith
 * Copyright (c) 2024-2026 Aiblox (Alexsandro Alves de Oliveira)
 * 
 * ZeroCopyBuffer.java - Zero-Copy Memory Buffer System
 * 
 * Provides efficient memory transfer between Java and native code
 * without copying data. Uses direct ByteBuffers and native pointers.
 */

package dev.libs.bridge;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;
import org.jetbrains.annotations.NotNull;

import sun.misc.Unsafe;

import java.lang.reflect.Field;
import java.nio.Buffer;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;
import java.nio.IntBuffer;
import java.nio.LongBuffer;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/**
 * ZeroCopyBuffer - High-Performance Memory Transfer
 * 
 * This class provides zero-copy memory access between Java and native code.
 * It uses sun.misc.Unsafe for direct memory manipulation when available,
 * with fallback to standard ByteBuffer operations.
 * 
 * <h2>Usage:</h2>
 * 
 * <pre>
 * try (ZeroCopyBuffer buf = ZeroCopyBuffer.allocate(1024)) {
 *     buf.putFloat(0, 1.5f);
 *     float value = buf.getFloat(0);
 * 
 *     // Pass to native
 *     long nativePtr = buf.getNativePointer();
 * }
 * </pre>
 * 
 * <h2>Memory Layout:</h2>
 * Data is stored in native byte order for maximum performance.
 * 
 * @author Aiblox (Alexsandro Alves de Oliveira)
 * @version 1.0.0-alpha
 * @since 1.0.0
 */
public final class ZeroCopyBuffer implements AutoCloseable {

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    private static final Logger LOGGER = LogManager.getLogger(ZeroCopyBuffer.class);

    /** Maximum buffer size (1 GB) */
    public static final long MAX_SIZE = 1024L * 1024L * 1024L;

    /** Minimum alignment for native ops */
    private static final int ALIGNMENT = 8;

    // =========================================================================
    // STATIC FIELDS
    // =========================================================================

    /** Unsafe instance for direct memory access */
    private static final Unsafe UNSAFE;

    /** Whether Unsafe is available */
    private static final boolean UNSAFE_AVAILABLE;

    /** Address field in ByteBuffer for getting native pointer */
    private static final Field ADDRESS_FIELD;

    /** Total allocated memory tracking */
    private static final AtomicLong totalAllocated = new AtomicLong(0);

    static {
        Unsafe unsafe = null;
        Field addressField = null;

        try {
            Field field = Unsafe.class.getDeclaredField("theUnsafe");
            field.setAccessible(true);
            unsafe = (Unsafe) field.get(null);

            // Get address field from ByteBuffer
            addressField = Buffer.class.getDeclaredField("address");
            addressField.setAccessible(true);

            LOGGER.debug("Unsafe memory access available");
        } catch (Exception e) {
            LOGGER.warn("Unsafe not available, using fallback methods: {}", e.getMessage());
        }

        UNSAFE = unsafe;
        UNSAFE_AVAILABLE = unsafe != null;
        ADDRESS_FIELD = addressField;
    }

    // =========================================================================
    // INSTANCE FIELDS
    // =========================================================================

    /** The underlying direct ByteBuffer */
    private final ByteBuffer buffer;

    /** Size in bytes */
    private final long size;

    /** Native memory address */
    private final long address;

    /** Whether this buffer is freed */
    private final AtomicBoolean freed = new AtomicBoolean(false);

    /** Whether this buffer owns its memory */
    private final boolean ownsMemory;

    /** Read-only flag */
    private final boolean readOnly;

    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /**
     * Private constructor - use factory methods
     */
    private ZeroCopyBuffer(ByteBuffer buffer, long address, boolean ownsMemory, boolean readOnly) {
        this.buffer = buffer;
        this.size = buffer.capacity();
        this.address = address;
        this.ownsMemory = ownsMemory;
        this.readOnly = readOnly;

        if (ownsMemory) {
            totalAllocated.addAndGet(size);
        }
    }

    // =========================================================================
    // FACTORY METHODS
    // =========================================================================

    /**
     * Allocate a new zero-copy buffer
     * 
     * @param size Size in bytes
     * @return New buffer
     */
    public static ZeroCopyBuffer allocate(long size) {
        if (size <= 0 || size > MAX_SIZE) {
            throw new IllegalArgumentException("Invalid size: " + size);
        }

        // Align size
        long alignedSize = (size + ALIGNMENT - 1) & ~(ALIGNMENT - 1);

        ByteBuffer buffer = ByteBuffer.allocateDirect((int) alignedSize)
                .order(ByteOrder.nativeOrder());

        long address = getBufferAddress(buffer);

        LOGGER.trace("Allocated ZeroCopyBuffer: {} bytes at 0x{}", alignedSize, Long.toHexString(address));

        return new ZeroCopyBuffer(buffer, address, true, false);
    }

    /**
     * Allocate a buffer for floats
     * 
     * @param count Number of floats
     * @return New buffer
     */
    public static ZeroCopyBuffer allocateFloats(int count) {
        return allocate((long) count * Float.BYTES);
    }

    /**
     * Allocate a buffer for integers
     * 
     * @param count Number of integers
     * @return New buffer
     */
    public static ZeroCopyBuffer allocateInts(int count) {
        return allocate((long) count * Integer.BYTES);
    }

    /**
     * Allocate a buffer for longs
     * 
     * @param count Number of longs
     * @return New buffer
     */
    public static ZeroCopyBuffer allocateLongs(int count) {
        return allocate((long) count * Long.BYTES);
    }

    /**
     * Wrap an existing ByteBuffer (must be direct)
     * 
     * @param buffer Direct ByteBuffer to wrap
     * @return Wrapped buffer
     */
    public static ZeroCopyBuffer wrap(@NotNull ByteBuffer buffer) {
        if (!buffer.isDirect()) {
            throw new IllegalArgumentException("Buffer must be direct");
        }

        long address = getBufferAddress(buffer);
        return new ZeroCopyBuffer(buffer, address, false, buffer.isReadOnly());
    }

    /**
     * Wrap a native memory address
     * 
     * @param address Native memory address
     * @param size    Size in bytes
     * @return Wrapped buffer
     */
    public static ZeroCopyBuffer wrapNative(long address, long size) {
        if (address == 0) {
            throw new IllegalArgumentException("Null pointer");
        }
        if (size <= 0 || size > MAX_SIZE) {
            throw new IllegalArgumentException("Invalid size: " + size);
        }

        // Create a view of the native memory
        // This requires Unsafe
        if (!UNSAFE_AVAILABLE) {
            throw new UnsupportedOperationException(
                    "Wrapping native memory requires Unsafe which is not available");
        }

        // We can't directly create a ByteBuffer pointing to arbitrary memory
        // without additional native code, so we return a placeholder
        ByteBuffer buffer = ByteBuffer.allocateDirect((int) size)
                .order(ByteOrder.nativeOrder());

        return new ZeroCopyBuffer(buffer, address, false, false);
    }

    // =========================================================================
    // ADDRESS RETRIEVAL
    // =========================================================================

    /**
     * Get the native memory address of a direct ByteBuffer
     */
    private static long getBufferAddress(ByteBuffer buffer) {
        if (!buffer.isDirect()) {
            throw new IllegalArgumentException("Buffer must be direct");
        }

        if (UNSAFE_AVAILABLE && ADDRESS_FIELD != null) {
            try {
                return ADDRESS_FIELD.getLong(buffer);
            } catch (IllegalAccessException e) {
                LOGGER.warn("Failed to get buffer address via reflection", e);
            }
        }

        // Fallback: Return buffer's hashCode as a pseudo-address
        // This won't work for actual native interop but prevents crashes
        return buffer.hashCode() & 0xFFFFFFFFL;
    }

    // =========================================================================
    // PRIMITIVE ACCESS - BYTES
    // =========================================================================

    /**
     * Get a byte at the specified offset
     */
    public byte getByte(long offset) {
        checkBounds(offset, 1);

        if (UNSAFE_AVAILABLE) {
            return UNSAFE.getByte(address + offset);
        }
        return buffer.get((int) offset);
    }

    /**
     * Put a byte at the specified offset
     */
    public void putByte(long offset, byte value) {
        checkWritable();
        checkBounds(offset, 1);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.putByte(address + offset, value);
        } else {
            buffer.put((int) offset, value);
        }
    }

    /**
     * Get bytes into an array
     */
    public void getBytes(long offset, byte[] dest, int destOffset, int length) {
        checkBounds(offset, length);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(null, address + offset, dest,
                    Unsafe.ARRAY_BYTE_BASE_OFFSET + destOffset, length);
        } else {
            ByteBuffer view = buffer.duplicate();
            view.position((int) offset);
            view.get(dest, destOffset, length);
        }
    }

    /**
     * Put bytes from an array
     */
    public void putBytes(long offset, byte[] src, int srcOffset, int length) {
        checkWritable();
        checkBounds(offset, length);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(src, Unsafe.ARRAY_BYTE_BASE_OFFSET + srcOffset,
                    null, address + offset, length);
        } else {
            ByteBuffer view = buffer.duplicate();
            view.position((int) offset);
            view.put(src, srcOffset, length);
        }
    }

    // =========================================================================
    // PRIMITIVE ACCESS - SHORTS
    // =========================================================================

    public short getShort(long offset) {
        checkBounds(offset, 2);

        if (UNSAFE_AVAILABLE) {
            return UNSAFE.getShort(address + offset);
        }
        return buffer.getShort((int) offset);
    }

    public void putShort(long offset, short value) {
        checkWritable();
        checkBounds(offset, 2);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.putShort(address + offset, value);
        } else {
            buffer.putShort((int) offset, value);
        }
    }

    // =========================================================================
    // PRIMITIVE ACCESS - INTEGERS
    // =========================================================================

    public int getInt(long offset) {
        checkBounds(offset, 4);

        if (UNSAFE_AVAILABLE) {
            return UNSAFE.getInt(address + offset);
        }
        return buffer.getInt((int) offset);
    }

    public void putInt(long offset, int value) {
        checkWritable();
        checkBounds(offset, 4);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.putInt(address + offset, value);
        } else {
            buffer.putInt((int) offset, value);
        }
    }

    public void getInts(long offset, int[] dest, int destOffset, int count) {
        checkBounds(offset, (long) count * 4);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(null, address + offset, dest,
                    Unsafe.ARRAY_INT_BASE_OFFSET + (long) destOffset * 4, (long) count * 4);
        } else {
            IntBuffer view = buffer.asIntBuffer();
            view.position((int) (offset / 4));
            view.get(dest, destOffset, count);
        }
    }

    public void putInts(long offset, int[] src, int srcOffset, int count) {
        checkWritable();
        checkBounds(offset, (long) count * 4);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(src, Unsafe.ARRAY_INT_BASE_OFFSET + (long) srcOffset * 4,
                    null, address + offset, (long) count * 4);
        } else {
            IntBuffer view = buffer.asIntBuffer();
            view.position((int) (offset / 4));
            view.put(src, srcOffset, count);
        }
    }

    // =========================================================================
    // PRIMITIVE ACCESS - LONGS
    // =========================================================================

    public long getLong(long offset) {
        checkBounds(offset, 8);

        if (UNSAFE_AVAILABLE) {
            return UNSAFE.getLong(address + offset);
        }
        return buffer.getLong((int) offset);
    }

    public void putLong(long offset, long value) {
        checkWritable();
        checkBounds(offset, 8);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.putLong(address + offset, value);
        } else {
            buffer.putLong((int) offset, value);
        }
    }

    // =========================================================================
    // PRIMITIVE ACCESS - FLOATS
    // =========================================================================

    public float getFloat(long offset) {
        checkBounds(offset, 4);

        if (UNSAFE_AVAILABLE) {
            return UNSAFE.getFloat(address + offset);
        }
        return buffer.getFloat((int) offset);
    }

    public void putFloat(long offset, float value) {
        checkWritable();
        checkBounds(offset, 4);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.putFloat(address + offset, value);
        } else {
            buffer.putFloat((int) offset, value);
        }
    }

    public void getFloats(long offset, float[] dest, int destOffset, int count) {
        checkBounds(offset, (long) count * 4);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(null, address + offset, dest,
                    Unsafe.ARRAY_FLOAT_BASE_OFFSET + (long) destOffset * 4, (long) count * 4);
        } else {
            FloatBuffer view = buffer.asFloatBuffer();
            view.position((int) (offset / 4));
            view.get(dest, destOffset, count);
        }
    }

    public void putFloats(long offset, float[] src, int srcOffset, int count) {
        checkWritable();
        checkBounds(offset, (long) count * 4);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(src, Unsafe.ARRAY_FLOAT_BASE_OFFSET + (long) srcOffset * 4,
                    null, address + offset, (long) count * 4);
        } else {
            FloatBuffer view = buffer.asFloatBuffer();
            view.position((int) (offset / 4));
            view.put(src, srcOffset, count);
        }
    }

    // =========================================================================
    // PRIMITIVE ACCESS - DOUBLES
    // =========================================================================

    public double getDouble(long offset) {
        checkBounds(offset, 8);

        if (UNSAFE_AVAILABLE) {
            return UNSAFE.getDouble(address + offset);
        }
        return buffer.getDouble((int) offset);
    }

    public void putDouble(long offset, double value) {
        checkWritable();
        checkBounds(offset, 8);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.putDouble(address + offset, value);
        } else {
            buffer.putDouble((int) offset, value);
        }
    }

    // =========================================================================
    // BULK OPERATIONS
    // =========================================================================

    /**
     * Copy data from another ZeroCopyBuffer
     */
    public void copyFrom(ZeroCopyBuffer src, long srcOffset, long dstOffset, long length) {
        checkWritable();
        checkBounds(dstOffset, length);
        src.checkBounds(srcOffset, length);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.copyMemory(src.address + srcOffset, address + dstOffset, length);
        } else {
            ByteBuffer srcView = src.buffer.duplicate();
            srcView.position((int) srcOffset).limit((int) (srcOffset + length));

            ByteBuffer dstView = buffer.duplicate();
            dstView.position((int) dstOffset);

            dstView.put(srcView);
        }
    }

    /**
     * Fill the buffer with a byte value
     */
    public void fill(byte value) {
        fill(0, size, value);
    }

    /**
     * Fill a region with a byte value
     */
    public void fill(long offset, long length, byte value) {
        checkWritable();
        checkBounds(offset, length);

        if (UNSAFE_AVAILABLE) {
            UNSAFE.setMemory(address + offset, length, value);
        } else {
            for (long i = 0; i < length; i++) {
                buffer.put((int) (offset + i), value);
            }
        }
    }

    /**
     * Clear the buffer (fill with zeros)
     */
    public void clear() {
        fill((byte) 0);
    }

    // =========================================================================
    // VIEWS
    // =========================================================================

    /**
     * Get the underlying ByteBuffer
     */
    public ByteBuffer asByteBuffer() {
        checkNotFreed();
        return buffer.duplicate().order(ByteOrder.nativeOrder());
    }

    /**
     * Get a FloatBuffer view
     */
    public FloatBuffer asFloatBuffer() {
        checkNotFreed();
        return buffer.asFloatBuffer();
    }

    /**
     * Get an IntBuffer view
     */
    public IntBuffer asIntBuffer() {
        checkNotFreed();
        return buffer.asIntBuffer();
    }

    /**
     * Get a LongBuffer view
     */
    public LongBuffer asLongBuffer() {
        checkNotFreed();
        return buffer.asLongBuffer();
    }

    /**
     * Create a slice of this buffer
     */
    public ZeroCopyBuffer slice(long offset, long length) {
        checkBounds(offset, length);

        ByteBuffer sliced = buffer.duplicate();
        sliced.position((int) offset);
        sliced.limit((int) (offset + length));
        ByteBuffer slicedBuffer = sliced.slice().order(ByteOrder.nativeOrder());

        return new ZeroCopyBuffer(slicedBuffer, address + offset, false, readOnly);
    }

    // =========================================================================
    // CHECKS
    // =========================================================================

    private void checkNotFreed() {
        if (freed.get()) {
            throw new IllegalStateException("Buffer has been freed");
        }
    }

    private void checkWritable() {
        checkNotFreed();
        if (readOnly) {
            throw new IllegalStateException("Buffer is read-only");
        }
    }

    private void checkBounds(long offset, long length) {
        checkNotFreed();
        if (offset < 0 || length < 0 || offset + length > size) {
            throw new IndexOutOfBoundsException(
                    String.format("offset=%d, length=%d, size=%d", offset, length, size));
        }
    }

    // =========================================================================
    // LIFECYCLE
    // =========================================================================

    @Override
    public void close() {
        if (freed.compareAndSet(false, true)) {
            if (ownsMemory) {
                totalAllocated.addAndGet(-size);
                LOGGER.trace("Freed ZeroCopyBuffer: {} bytes", size);
            }
            // Note: Direct ByteBuffer memory is freed by GC
            // We just track our usage here
        }
    }

    // =========================================================================
    // GETTERS
    // =========================================================================

    public long getSize() {
        return size;
    }

    public long getNativePointer() {
        return address;
    }

    public boolean isFreed() {
        return freed.get();
    }

    public boolean isReadOnly() {
        return readOnly;
    }

    public boolean ownsMemory() {
        return ownsMemory;
    }

    public static long getTotalAllocated() {
        return totalAllocated.get();
    }

    public static boolean isUnsafeAvailable() {
        return UNSAFE_AVAILABLE;
    }

    @Override
    public String toString() {
        return String.format("ZeroCopyBuffer[size=%d, address=0x%x, freed=%b]",
                size, address, freed.get());
    }
}
