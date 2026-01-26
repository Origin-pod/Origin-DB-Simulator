//! Core block abstractions and types
//!
//! This module defines the fundamental block types, traits, and metadata structures
//! that form the foundation of the block system.

pub mod metrics;
pub mod constraint;
pub mod port;
pub mod parameter;
pub mod block;
pub mod registry;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub Uuid);

impl BlockId {
    /// Generate a new random block ID
    pub fn new() -> Self {
        BlockId(Uuid::new_v4())
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetadata {
    pub id: BlockId,
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
}

/// Core block trait defining the interface for all blocks
pub trait Block: Send + Sync {
    /// Get the block's unique identifier
    fn id(&self) -> BlockId;

    /// Get the block's metadata
    fn metadata(&self) -> &BlockMetadata;

    /// Validate the block's configuration
    fn validate(&self) -> Result<(), String>;

    /// Clone the block as a trait object
    fn clone_box(&self) -> Box<dyn Block>;
}

impl Clone for Box<dyn Block> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
