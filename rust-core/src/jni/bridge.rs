//! # JNI Bridge Implementation
//! 
//! Contains the actual JNI native method implementations that are called
//! from Java's NativeBridge class.
//! 
//! ## Function Naming Convention
//! 
//! JNI functions must follow this pattern:
//! `Java_<package>_<class>_<method>`
//! 
//! For example:
//! `Java_dev_libs_bridge_NativeBridge_nativeCreateEngine`

use jni::JNIEnv;
use jni::objects::{JClass, JObject, JString, JByteArray, JByteBuffer};
use jni::sys::{jboolean, jint, jlong, jfloat, jdouble, JNI_TRUE, JNI_FALSE};

use crate::engine::AetherEngine;
use crate::memory::MemoryManager;

// ============================================================================
// LIFECYCLE FUNCTIONS
// ============================================================================

/// Create the native engine instance
#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeCreateEngine(
    mut env: JNIEnv,
    _class: JClass,
    config: JByteArray,
) -> jlong {
    log::info!("JNI: nativeCreateEngine called");
    
    // Parse config from byte array
    let config_bytes = match env.convert_byte_array(config) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Failed to read config bytes: {}", e);
            return 0;
        }
    };
    
    // Create engine instance
    match AetherEngine::new(&config_bytes) {
        Ok(engine) => {
            let engine_ptr = Box::into_raw(Box::new(engine));
            log::info!("Engine created at 0x{:p}", engine_ptr);
            engine_ptr as jlong
        }
        Err(e) => {
            log::error!("Failed to create engine: {}", e);
            0
        }
    }
}

/// Destroy the native engine instance
#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeDestroyEngine(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        // Drop the box to free memory
        let _ = Box::from_raw(engine_ptr);
        log::info!("Engine destroyed");
    }
}

/// Initialize the native engine
#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeInitialize(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    _hardware_info: JByteArray,
    _vulkan_caps: JByteArray,
    _config: JByteArray,
) -> jboolean {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return JNI_FALSE;
    }

    log::info!("JNI: nativeInitialize called");
    
    // Initialize the library if not already done
    crate::initialize();
    
    JNI_TRUE
}

/// Shutdown the native engine
#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeShutdown(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).shutdown();
    }
}

/// Pause the native engine
#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativePause(
    _env: JNIEnv,
    _class: JClass,
    _handle: jlong,
) {
    log::debug!("JNI: nativePause called");
}

/// Resume the native engine
#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeResume(
    _env: JNIEnv,
    _class: JClass,
    _handle: jlong,
) {
    log::debug!("JNI: nativeResume called");
}

// ============================================================================
// VERSION INFO
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeGetVersion<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    match env.new_string(crate::VERSION) {
        Ok(s) => s,
        Err(_) => env.new_string("unknown").unwrap(),
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeGetBuildTime(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    // Build timestamp - would be injected at compile time
    chrono::Utc::now().timestamp()
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeGetApiVersion(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    1 // API Version 1
}

// ============================================================================
// TICK FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeOnTick(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    _tick: jlong,
    delta_time: jfloat,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).tick(delta_time);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativePrepareFrame(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    partial_ticks: jfloat,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).begin_frame(partial_ticks);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeRenderWorld(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    _frame: jlong,
    _partial_ticks: jfloat,
    _view_matrix: JObject,
    _proj_matrix: JObject,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        // Rendering would be handled by the Vulkan renderer
        // View/projection matrices would be extracted from the buffers
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeCompositeFrame(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).end_frame();
    }
}

// ============================================================================
// MEMORY FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeAllocate(
    _env: JNIEnv,
    _class: JClass,
    size: jlong,
) -> jlong {
    match MemoryManager::allocate(size as usize) {
        Some(ptr) => ptr as jlong,
        None => 0
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeFree(
    _env: JNIEnv,
    _class: JClass,
    pointer: jlong,
) {
    if pointer != 0 {
        MemoryManager::free(pointer as *mut u8);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeSyncMemory(
    _env: JNIEnv,
    _class: JClass,
    _handle: jlong,
) {
    // Memory synchronization - flush caches if needed
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeGetMemoryUsage(
    _env: JNIEnv,
    _class: JClass,
    _handle: jlong,
) -> jlong {
    MemoryManager::get_allocated_bytes() as jlong
}

// ============================================================================
// ENTITY FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeSpawnEntity(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    entity_id: jint,
    _entity_type_str: JString,
    x: jdouble,
    y: jdouble,
    z: jdouble,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    (*engine_ptr).register_entity(entity_id, 0, x, y, z);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeRemoveEntity(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    entity_id: jint,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).remove_entity(entity_id as u64);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeUpdateEntity(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    entity_id: jint,
    x: jdouble,
    y: jdouble,
    z: jdouble,
    yaw: jfloat,
    pitch: jfloat,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).update_entity(entity_id as u64, x, y, z, yaw, pitch);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeBatchUpdateEntities(
    _env: JNIEnv,
    _class: JClass,
    _handle: jlong,
    _entity_ids: JObject,
    _positions: JObject,
    _count: jint,
) {
    // Batch update implementation using direct buffers
    // Would extract data from IntBuffer and FloatBuffer
}

// ============================================================================
// CHUNK FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeLoadChunk(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    x: jint,
    z: jint,
    data: JByteBuffer,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    // Get direct buffer address
    let addr = match env.get_direct_buffer_address(&data) {
        Ok(ptr) => ptr,
        Err(_) => return,
    };
    
    // Get direct buffer capacity
    let len = match env.get_direct_buffer_capacity(&data) {
        Ok(l) => l,
        Err(_) => return,
    };
    
    let slice = std::slice::from_raw_parts(addr, len);
    (*engine_ptr).submit_chunk(x, z, slice);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeUnloadChunk(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    x: jint,
    z: jint,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).unload_chunk(x, z);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeUpdateChunk(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    x: jint,
    z: jint,
    data: JByteBuffer,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    // Get direct buffer address
    let addr = match env.get_direct_buffer_address(&data) {
        Ok(ptr) => ptr,
        Err(_) => return,
    };
    
    let len = match env.get_direct_buffer_capacity(&data) {
        Ok(l) => l,
        Err(_) => return,
    };
    
    let slice = std::slice::from_raw_parts(addr, len);
    (*engine_ptr).update_chunk(x, z, slice);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeMarkChunkDirty(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    x: jint,
    z: jint,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).mark_chunk_dirty(x, z);
    }
}

// ============================================================================
// TEXTURE FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeUploadTexture(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    name: JString,
    data: JByteBuffer,
    width: jint,
    height: jint,
    format: jint,
) -> jlong {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return 0;
    }
    
    let texture_name: String = match env.get_string(&name) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    
    let addr = match env.get_direct_buffer_address(&data) {
        Ok(ptr) => ptr,
        Err(_) => return 0,
    };
    
    let len = match env.get_direct_buffer_capacity(&data) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    
    let slice = std::slice::from_raw_parts(addr, len);
    (*engine_ptr).upload_texture(&texture_name, slice, width as u32, height as u32, format as u32)
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeUnloadTexture(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    texture_handle: jlong,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).unload_texture(texture_handle as u64);
    }
}

// ============================================================================
// AUDIO FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativePlaySound(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    sound_id: JString,
    x: jfloat,
    y: jfloat,
    z: jfloat,
    volume: jfloat,
    pitch: jfloat,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    let sound_name: String = match env.get_string(&sound_id) {
        Ok(s) => s.into(),
        Err(_) => return,
    };
    
    (*engine_ptr).play_sound(&sound_name, x, y, z, volume, pitch);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeStopSound(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    sound: JString,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    let sound_name: String = match env.get_string(&sound) {
        Ok(s) => s.into(),
        Err(_) => return,
    };
    
    (*engine_ptr).stop_sound_by_name(&sound_name);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeUpdateListener(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    x: jfloat,
    y: jfloat,
    z: jfloat,
    yaw: jfloat,
    pitch: jfloat,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if !engine_ptr.is_null() {
        (*engine_ptr).update_listener(x, y, z, yaw, pitch);
    }
}

// ============================================================================
// NETWORK FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeCompressPacket<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    _handle: jlong,
    data: JByteArray,
) -> JByteArray<'local> {
    let input = match env.convert_byte_array(data) {
        Ok(bytes) => bytes,
        Err(_) => return JByteArray::default(),
    };
    
    // Use zstd compression
    let compressed = match crate::network::compress(&input) {
        Ok(c) => c,
        Err(_) => return JByteArray::default(),
    };
    
    match env.byte_array_from_slice(&compressed) {
        Ok(arr) => arr,
        Err(_) => JByteArray::default(),
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeDecompressPacket<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    _handle: jlong,
    data: JByteArray,
) -> JByteArray<'local> {
    let input = match env.convert_byte_array(data) {
        Ok(bytes) => bytes,
        Err(_) => return JByteArray::default(),
    };
    
    // Use zstd decompression
    let decompressed = match crate::network::decompress(&input) {
        Ok(d) => d,
        Err(_) => return JByteArray::default(),
    };
    
    match env.byte_array_from_slice(&decompressed) {
        Ok(arr) => arr,
        Err(_) => JByteArray::default(),
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativePredictState(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    tick: jlong,
    state: JByteBuffer,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    let addr = match env.get_direct_buffer_address(&state) {
        Ok(ptr) => ptr,
        Err(_) => return,
    };
    
    let len = match env.get_direct_buffer_capacity(&state) {
        Ok(l) => l,
        Err(_) => return,
    };
    
    let slice = std::slice::from_raw_parts(addr, len);
    (*engine_ptr).predict_state(tick as u64, slice);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeReconcileState(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    tick: jlong,
    server_state: JByteBuffer,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    let addr = match env.get_direct_buffer_address(&server_state) {
        Ok(ptr) => ptr,
        Err(_) => return,
    };
    
    let len = match env.get_direct_buffer_capacity(&server_state) {
        Ok(l) => l,
        Err(_) => return,
    };
    
    let slice = std::slice::from_raw_parts(addr, len);
    (*engine_ptr).reconcile_state(tick as u64, slice);
}

// ============================================================================
// DEBUG FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeGetDebugInfo<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return env.new_string("Engine not initialized").unwrap();
    }
    
    let info = (*engine_ptr).get_debug_info();
    match env.new_string(&info) {
        Ok(s) => s,
        Err(_) => env.new_string("Error getting debug info").unwrap(),
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeSetDebugFlag(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    flag: JString,
    value: jboolean,
) {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return;
    }
    
    let flag_name: String = match env.get_string(&flag) {
        Ok(s) => s.into(),
        Err(_) => return,
    };
    
    (*engine_ptr).set_debug_flag(&flag_name, value != 0);
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeGetProfileData(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jlong {
    let engine_ptr = handle as *mut AetherEngine;
    if engine_ptr.is_null() {
        return 0;
    }
    
    (*engine_ptr).get_profile_data_ptr()
}

// ============================================================================
// CALLBACK FUNCTIONS
// ============================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_dev_libs_bridge_NativeBridge_nativeRegisterCallbacks(
    mut env: JNIEnv,
    _class: JClass,
    _handle: jlong,
    callback_handler: JObject,
) {
    log::info!("JNI: nativeRegisterCallbacks called");
    
    // Initialize callback system
    if let Err(e) = crate::jni::callback::init(&mut env, callback_handler) {
        log::error!("Failed to register callbacks: {}", e);
    } else {
        log::info!("JNI callbacks registered successfully");
    }
}
