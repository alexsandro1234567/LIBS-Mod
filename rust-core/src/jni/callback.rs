//! # JNI Callback System
//! 
//! Handles callbacks from Rust back to Java.

use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject, JValue};
use std::sync::RwLock;

/// Global callback handler reference
static CALLBACK_HANDLER: RwLock<Option<GlobalRef>> = RwLock::new(None);

/// Initialize callback system with Java callback handler
pub fn init(env: &mut JNIEnv, handler: JObject) -> Result<(), String> {
    let global_ref = env.new_global_ref(handler)
        .map_err(|e| format!("Failed to create global ref: {}", e))?;
    
    let mut guard = CALLBACK_HANDLER.write()
        .map_err(|e| format!("Lock error: {}", e))?;
    
    *guard = Some(global_ref);
    
    log::debug!("Callback handler initialized");
    Ok(())
}

/// Shutdown callback system
pub fn shutdown() {
    if let Ok(mut guard) = CALLBACK_HANDLER.write() {
        *guard = None;
    }
}

/// Callback types matching Java constants
pub mod callback_type {
    pub const ERROR: i32 = 0;
    pub const LOG: i32 = 1;
    pub const FRAME_READY: i32 = 10;
    pub const CHUNK_READY: i32 = 20;
    pub const CHUNK_MESH_READY: i32 = 21;
    pub const ENTITY_UPDATE: i32 = 30;
    pub const ENTITY_REMOVED: i32 = 31;
    pub const SOUND_COMPLETE: i32 = 40;
    pub const NETWORK_PACKET: i32 = 50;
    pub const MEMORY_WARNING: i32 = 60;
    pub const MEMORY_CRITICAL: i32 = 61;
    pub const PROFILING_DATA: i32 = 70;
}

/// Send error callback to Java
pub fn send_error(env: &mut JNIEnv, code: i32, message: &str, details: &str) {
    if let Ok(guard) = CALLBACK_HANDLER.read() {
        if let Some(ref handler) = *guard {
            let j_message = match env.new_string(message) {
                Ok(s) => s,
                Err(_) => return,
            };
            let j_details = match env.new_string(details) {
                Ok(s) => s,
                Err(_) => return,
            };
            
            let _ = env.call_method(
                handler.as_obj(),
                "onError",
                "(ILjava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Int(code),
                    JValue::Object(&j_message),
                    JValue::Object(&j_details),
                ],
            );
        }
    }
}

/// Send log callback to Java
pub fn send_log(env: &mut JNIEnv, level: i32, module: &str, message: &str) {
    if let Ok(guard) = CALLBACK_HANDLER.read() {
        if let Some(ref handler) = *guard {
            let j_module = match env.new_string(module) {
                Ok(s) => s,
                Err(_) => return,
            };
            let j_message = match env.new_string(message) {
                Ok(s) => s,
                Err(_) => return,
            };
            
            let _ = env.call_method(
                handler.as_obj(),
                "onLog",
                "(ILjava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Int(level),
                    JValue::Object(&j_module),
                    JValue::Object(&j_message),
                ],
            );
        }
    }
}

/// Send chunk complete callback to Java
pub fn send_chunk_complete(env: &mut JNIEnv, x: i32, z: i32, handle: i64, vertices: i32, indices: i32) {
    if let Ok(guard) = CALLBACK_HANDLER.read() {
        if let Some(ref handler) = *guard {
            let _ = env.call_method(
                handler.as_obj(),
                "onChunkComplete",
                "(IIJII)V",
                &[
                    JValue::Int(x),
                    JValue::Int(z),
                    JValue::Long(handle),
                    JValue::Int(vertices),
                    JValue::Int(indices),
                ],
            );
        }
    }
}

/// Send simple callback (just type and value)
pub fn send_simple(env: &mut JNIEnv, callback_type: i32, value: i32) {
    if let Ok(guard) = CALLBACK_HANDLER.read() {
        if let Some(ref handler) = *guard {
            let _ = env.call_method(
                handler.as_obj(),
                "onSimpleCallback",
                "(II)V",
                &[
                    JValue::Int(callback_type),
                    JValue::Int(value),
                ],
            );
        }
    }
}
