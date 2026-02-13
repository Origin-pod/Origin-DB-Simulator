//! Block System - Core implementation for Modular DB Builder
//!
//! This crate provides the foundational block system for the Modular DB Builder,
//! including core abstractions, block categories, and runtime execution engine.

pub mod core;
pub mod categories;
pub mod runtime;
mod tests;

#[cfg(target_arch = "wasm32")]
pub mod wasm_api;

// Re-export commonly used types
pub use core::{Block, BlockId, BlockMetadata};
pub use categories::BlockCategory;
pub use runtime::BlockRuntime;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
