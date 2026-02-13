//! Block trait and related types
//!
//! This module defines the core Block trait that all blocks must implement,
//! along with supporting types for metadata, documentation, execution context,
//! and error handling.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::parameter::{Parameter, ParameterValue, ValidationResult};
use super::port::{Port, PortValue};
use super::constraint::{Constraint, Guarantee};
use super::metrics::{MetricDefinition, MetricsCollector, Logger, StorageContext};

/// Core block trait that all blocks must implement
#[async_trait]
pub trait Block: Send + Sync {
    /// Get block metadata
    fn metadata(&self) -> &BlockMetadata;

    /// Get input port definitions
    fn inputs(&self) -> &[Port];

    /// Get output port definitions
    fn outputs(&self) -> &[Port];

    /// Get parameter definitions
    fn parameters(&self) -> &[Parameter];

    /// Get constraints this block requires
    fn requires(&self) -> &[Constraint];

    /// Get guarantees this block provides
    fn guarantees(&self) -> &[Guarantee];

    /// Get metric definitions
    fn metrics(&self) -> &[MetricDefinition];

    /// Initialize the block with parameters
    async fn initialize(&mut self, params: HashMap<String, ParameterValue>) -> Result<(), BlockError>;

    /// Execute the block with given context
    async fn execute(&mut self, context: ExecutionContext) -> Result<ExecutionResult, BlockError>;

    /// Validate inputs before execution
    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult;

    /// Get current block state (for serialization/debugging)
    fn get_state(&self) -> BlockState;

    /// Set block state (for deserialization/recovery)
    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError>;

    /// Lifecycle hook: called when block starts
    async fn on_start(&mut self) -> Result<(), BlockError> {
        Ok(())
    }

    /// Lifecycle hook: called when block stops
    async fn on_stop(&mut self) -> Result<(), BlockError> {
        Ok(())
    }

    /// Lifecycle hook: called when block resets
    async fn on_reset(&mut self) -> Result<(), BlockError> {
        Ok(())
    }
}

/// Block metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetadata {
    /// Unique block identifier
    pub id: String,
    /// Human-readable block name
    pub name: String,
    /// Block category
    pub category: BlockCategory,
    /// Brief description of the block
    pub description: String,
    /// Block version
    pub version: String,
    /// Detailed documentation
    pub documentation: BlockDocumentation,
    /// References to papers, books, etc.
    pub references: Vec<Reference>,
    /// Icon identifier
    pub icon: String,
    /// Color for UI representation
    pub color: String,
}

/// Block categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockCategory {
    /// Storage blocks (heap files, B-trees, etc.)
    Storage,
    /// Index structures
    Index,
    /// Concurrency control mechanisms
    Concurrency,
    /// Buffer management
    Buffer,
    /// Query execution
    Execution,
    /// Transaction management
    Transaction,
    /// Compression algorithms
    Compression,
    /// Partitioning strategies
    Partitioning,
    /// Query optimization
    Optimization,
    /// Distributed systems
    Distribution,
}

/// Block documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDocumentation {
    /// Overview of the block's purpose
    pub overview: String,
    /// Algorithm description
    pub algorithm: String,
    /// Complexity analysis
    pub complexity: Complexity,
    /// Common use cases
    pub use_cases: Vec<String>,
    /// Tradeoffs to consider
    pub tradeoffs: Vec<String>,
    /// Usage examples
    pub examples: Vec<String>,
    /// Why this component exists — what problem it solves
    pub motivation: String,
    /// Deep explanation per parameter (param_id → guide text)
    pub parameter_guide: HashMap<String, String>,
    /// Comparisons with alternative/sibling block types
    pub alternatives: Vec<Alternative>,
    /// Suggested AI chat starter questions
    pub suggested_questions: Vec<String>,
}

/// A comparison with an alternative block type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    /// The block_type identifier of the alternative (e.g., "hash_index")
    pub block_type: String,
    /// Comparison text explaining when to choose one vs the other
    pub comparison: String,
}

/// Complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Complexity {
    /// Time complexity (e.g., "O(log n)")
    pub time: String,
    /// Space complexity (e.g., "O(n)")
    pub space: String,
}

/// Reference to external resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// Type of reference
    pub ref_type: ReferenceType,
    /// Title of the reference
    pub title: String,
    /// URL if available
    pub url: Option<String>,
    /// Citation information
    pub citation: Option<String>,
}

/// Types of references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Academic paper
    Paper,
    /// Book
    Book,
    /// Blog post or article
    Blog,
    /// Implementation reference
    Implementation,
}

/// Block execution context
pub struct ExecutionContext {
    /// Input port values
    pub inputs: HashMap<String, PortValue>,
    /// Parameter values
    pub parameters: HashMap<String, ParameterValue>,
    /// Metrics collector
    pub metrics: MetricsCollector,
    /// Logger for debugging
    pub logger: Logger,
    /// Storage context
    pub storage: StorageContext,
}

/// Block execution result
pub struct ExecutionResult {
    /// Output port values
    pub outputs: HashMap<String, PortValue>,
    /// Collected metrics
    pub metrics: HashMap<String, f64>,
    /// Any errors that occurred (non-fatal)
    pub errors: Vec<BlockError>,
}

/// Block state for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockState {
    /// Arbitrary state data
    pub data: HashMap<String, serde_json::Value>,
}

impl BlockState {
    /// Create a new empty block state
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert a value into the state
    pub fn insert<T: Serialize>(&mut self, key: String, value: T) -> Result<(), serde_json::Error> {
        let json_value = serde_json::to_value(value)?;
        self.data.insert(key, json_value);
        Ok(())
    }

    /// Get a value from the state
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, serde_json::Error> {
        match self.data.get(key) {
            Some(value) => {
                let result = serde_json::from_value(value.clone())?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
}

impl Default for BlockState {
    fn default() -> Self {
        Self::new()
    }
}

/// Block errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlockError {
    /// Initialization failed
    #[error("Initialization failed: {0}")]
    InitializationError(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionError(String),

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationError(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// State error
    #[error("State error: {0}")]
    StateError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for BlockError {
    fn from(error: std::io::Error) -> Self {
        BlockError::IoError(error.to_string())
    }
}
