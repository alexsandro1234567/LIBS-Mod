//! # JNI Type Conversions
//! 
//! Helper types and conversions for JNI interop.

use jni::sys::{jboolean, JNI_TRUE, JNI_FALSE};

/// Convert Rust bool to JNI boolean
#[inline]
pub fn to_jboolean(b: bool) -> jboolean {
    if b { JNI_TRUE } else { JNI_FALSE }
}

/// Convert JNI boolean to Rust bool
#[inline]
pub fn from_jboolean(b: jboolean) -> bool {
    b == JNI_TRUE
}

/// A handle that can be passed to/from Java
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NativeHandle(pub u64);

impl NativeHandle {
    pub const INVALID: NativeHandle = NativeHandle(0);
    
    pub fn new(value: u64) -> Self {
        NativeHandle(value)
    }
    
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
    
    pub fn to_jlong(&self) -> i64 {
        self.0 as i64
    }
    
    pub fn from_jlong(value: i64) -> Self {
        NativeHandle(value as u64)
    }
}

impl From<u64> for NativeHandle {
    fn from(value: u64) -> Self {
        NativeHandle(value)
    }
}

impl From<NativeHandle> for u64 {
    fn from(handle: NativeHandle) -> Self {
        handle.0
    }
}

/// Result type for JNI operations
pub type JniResult<T> = Result<T, JniError>;

/// JNI error type
#[derive(Debug)]
pub enum JniError {
    NullPointer(&'static str),
    InvalidHandle,
    StringConversion,
    ArrayConversion,
    MethodInvocation(String),
    Other(String),
}

impl std::fmt::Display for JniError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JniError::NullPointer(name) => write!(f, "Null pointer: {}", name),
            JniError::InvalidHandle => write!(f, "Invalid native handle"),
            JniError::StringConversion => write!(f, "String conversion failed"),
            JniError::ArrayConversion => write!(f, "Array conversion failed"),
            JniError::MethodInvocation(msg) => write!(f, "Method invocation failed: {}", msg),
            JniError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for JniError {}
