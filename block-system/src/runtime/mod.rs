//! Block runtime execution engine
//!
//! This module provides the runtime system for executing blocks and managing
//! the data flow between blocks in a pipeline.

pub mod engine;
pub mod validation;
pub mod workload;

use crate::core::BlockId;
use std::collections::HashMap;

/// Block runtime execution engine
pub struct BlockRuntime {
    blocks: HashMap<BlockId, Box<dyn crate::core::Block>>,
}

impl BlockRuntime {
    /// Create a new block runtime
    pub fn new() -> Self {
        BlockRuntime {
            blocks: HashMap::new(),
        }
    }

    /// Register a block in the runtime
    pub fn register_block(&mut self, block: Box<dyn crate::core::Block>) -> Result<(), String> {
        let id = block.id();
        block.validate()?;
        self.blocks.insert(id, block);
        Ok(())
    }

    /// Get a block by ID
    pub fn get_block(&self, id: BlockId) -> Option<&(dyn crate::core::Block)> {
        self.blocks.get(&id).map(|b| b.as_ref())
    }

    /// Get the number of registered blocks
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

impl Default for BlockRuntime {
    fn default() -> Self {
        Self::new()
    }
}
