//! # JNI Bindings for Aether Core
//! 
//! This module provides the JNI (Java Native Interface) bindings
//! that allow Java code to call into the Rust native engine.
//! 
//! ## Safety
//! 
//! All JNI functions are inherently unsafe as they deal with raw pointers
//! from the JVM. Care is taken to validate all inputs and handle errors.

pub mod bridge;
pub mod callback;
pub mod types;

pub use bridge::*;
