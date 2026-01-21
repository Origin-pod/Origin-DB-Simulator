# Core Components - Rust Implementation

## 1. Block Trait (`core/block.rs`)

### Base Block Trait

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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
    pub id: String,
    pub name: String,
    pub category: BlockCategory,
    pub description: String,
    pub version: String,
    pub documentation: BlockDocumentation,
    pub references: Vec<Reference>,
    pub icon: String,
    pub color: String,
}

/// Block categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockCategory {
    Storage,
    Index,
    Concurrency,
    Buffer,
    Execution,
    Transaction,
    Compression,
    Partitioning,
    Optimization,
    Distribution,
}

/// Block documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDocumentation {
    pub overview: String,
    pub algorithm: String,
    pub complexity: Complexity,
    pub use_cases: Vec<String>,
    pub tradeoffs: Vec<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Complexity {
    pub time: String,
    pub space: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub ref_type: ReferenceType,
    pub title: String,
    pub url: Option<String>,
    pub citation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceType {
    Paper,
    Book,
    Blog,
    Implementation,
}

/// Block execution context
pub struct ExecutionContext {
    pub inputs: HashMap<String, PortValue>,
    pub parameters: HashMap<String, ParameterValue>,
    pub metrics: MetricsCollector,
    pub logger: Logger,
    pub storage: StorageContext,
}

/// Block execution result
pub struct ExecutionResult {
    pub outputs: HashMap<String, PortValue>,
    pub metrics: HashMap<String, f64>,
    pub errors: Vec<BlockError>,
}

/// Block state for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockState {
    pub data: HashMap<String, serde_json::Value>,
}

/// Block errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlockError {
    #[error("Initialization failed: {0}")]
    InitializationError(String),

    #[error("Execution failed: {0}")]
    ExecutionError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

---

## 2. Port System (`core/port.rs`)

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Port definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub id: String,
    pub name: String,
    pub port_type: PortType,
    pub direction: PortDirection,
    pub required: bool,
    pub multiple: bool,
    pub description: String,
    pub schema: Option<PortSchema>,
}

/// Port direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDirection {
    Input,
    Output,
}

/// Port data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    // Data types
    DataStream,      // Stream of records
    SingleValue,     // Single value
    Batch,           // Batch of records

    // Control signals
    Signal,          // Control signal
    Transaction,     // Transaction context

    // Metadata
    Schema,          // Schema information
    Statistics,      // Statistics data

    // Configuration
    Config,          // Configuration object
}

/// Port schema for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PortSchema {
    Object {
        properties: HashMap<String, Box<PortSchema>>,
        required: Vec<String>,
    },
    Array {
        items: Box<PortSchema>,
    },
    Primitive {
        prim_type: PrimitiveType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    Integer,
    Float,
    String,
    Boolean,
    Bytes,
}

/// Port value - actual data flowing through ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortValue {
    Stream(Vec<Record>),
    Single(Record),
    Batch(Vec<Record>),
    Signal(SignalValue),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub data: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalValue {
    Start,
    Stop,
    Commit,
    Abort,
    Custom(String),
}

/// Connection between ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub source_block_id: String,
    pub source_port_id: String,
    pub target_block_id: String,
    pub target_port_id: String,
    pub backpressure: bool,
    pub buffer_size: Option<usize>,
}

/// Port validator trait
pub trait PortValidator: Send + Sync {
    fn validate(&self, value: &PortValue) -> ValidationResult;
}
```

---

## 3. Parameter System (`core/parameter.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub id: String,
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub default_value: ParameterValue,
    pub required: bool,
    pub constraints: Option<ParameterConstraints>,
    pub ui_hint: Option<ParameterUIHint>,
}

/// Parameter types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Enum,
    Object,
    Array,
}

/// Parameter value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<ParameterValue>),
    Object(HashMap<String, ParameterValue>),
    Null,
}

/// Parameter constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraints {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<ParameterValue>>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
}

/// UI hints for parameter rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterUIHint {
    pub widget: WidgetType,
    pub step: Option<f64>,
    pub unit: Option<String>,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetType {
    Input,
    Slider,
    Select,
    Checkbox,
    Textarea,
    JsonEditor,
}

/// Parameter validator trait
pub trait ParameterValidator: Send + Sync {
    fn validate(
        &self,
        value: &ParameterValue,
        all_params: &HashMap<String, ParameterValue>,
    ) -> ValidationResult;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![msg.into()],
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, msg: impl Into<String>) -> Self {
        self.warnings.push(msg.into());
        self
    }
}
```

---

## 4. Block Registry (`core/registry.rs`)

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Block registry for managing all available blocks
pub struct BlockRegistry {
    blocks: Arc<RwLock<HashMap<String, Arc<dyn Block>>>>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new block
    pub fn register(&self, block: Arc<dyn Block>) -> Result<(), RegistryError> {
        let mut blocks = self.blocks.write()
            .map_err(|_| RegistryError::LockError)?;

        let id = block.metadata().id.clone();

        if blocks.contains_key(&id) {
            return Err(RegistryError::DuplicateBlock(id));
        }

        // Validate block before registration
        self.validate_block(&*block)?;

        blocks.insert(id, block);
        Ok(())
    }

    /// Unregister a block
    pub fn unregister(&self, block_id: &str) -> Result<(), RegistryError> {
        let mut blocks = self.blocks.write()
            .map_err(|_| RegistryError::LockError)?;

        blocks.remove(block_id)
            .ok_or_else(|| RegistryError::BlockNotFound(block_id.to_string()))?;

        Ok(())
    }

    /// Get a block by ID
    pub fn get_block(&self, id: &str) -> Result<Arc<dyn Block>, RegistryError> {
        let blocks = self.blocks.read()
            .map_err(|_| RegistryError::LockError)?;

        blocks.get(id)
            .cloned()
            .ok_or_else(|| RegistryError::BlockNotFound(id.to_string()))
    }

    /// Get all blocks
    pub fn get_all_blocks(&self) -> Result<Vec<Arc<dyn Block>>, RegistryError> {
        let blocks = self.blocks.read()
            .map_err(|_| RegistryError::LockError)?;

        Ok(blocks.values().cloned().collect())
    }

    /// Get blocks by category
    pub fn get_blocks_by_category(&self, category: BlockCategory) -> Result<Vec<Arc<dyn Block>>, RegistryError> {
        let blocks = self.blocks.read()
            .map_err(|_| RegistryError::LockError)?;

        Ok(blocks.values()
            .filter(|b| b.metadata().category == category)
            .cloned()
            .collect())
    }

    /// Search blocks by query
    pub fn search_blocks(&self, query: &str) -> Result<Vec<Arc<dyn Block>>, RegistryError> {
        let blocks = self.blocks.read()
            .map_err(|_| RegistryError::LockError)?;

        let query = query.to_lowercase();

        Ok(blocks.values()
            .filter(|b| {
                let meta = b.metadata();
                meta.name.to_lowercase().contains(&query) ||
                meta.description.to_lowercase().contains(&query)
            })
            .cloned()
            .collect())
    }

    /// Validate a block
    fn validate_block(&self, block: &dyn Block) -> Result<(), RegistryError> {
        let meta = block.metadata();

        // Validate metadata
        if meta.id.is_empty() {
            return Err(RegistryError::ValidationError("Block ID cannot be empty".into()));
        }

        if meta.name.is_empty() {
            return Err(RegistryError::ValidationError("Block name cannot be empty".into()));
        }

        // Validate ports
        for port in block.inputs() {
            if port.id.is_empty() {
                return Err(RegistryError::ValidationError(
                    format!("Input port ID cannot be empty in block {}", meta.id)
                ));
            }
        }

        for port in block.outputs() {
            if port.id.is_empty() {
                return Err(RegistryError::ValidationError(
                    format!("Output port ID cannot be empty in block {}", meta.id)
                ));
            }
        }

        Ok(())
    }

    /// Resolve dependencies for a set of blocks
    pub fn resolve_dependencies(&self, block_ids: &[String]) -> Result<DependencyGraph, RegistryError> {
        // Build dependency graph
        let mut graph = DependencyGraph::new();

        for block_id in block_ids {
            let block = self.get_block(block_id)?;
            graph.add_node(block_id.clone());

            // Add dependencies based on constraints
            for constraint in block.requires() {
                if let ConstraintType::RequiresBlock(dep_id) = &constraint.constraint_type {
                    graph.add_edge(block_id.clone(), dep_id.clone());
                }
            }
        }

        // Detect cycles
        graph.detect_cycles();

        Ok(graph)
    }

    /// Check compatibility between blocks
    pub fn check_compatibility(&self, block_ids: &[String]) -> Result<CompatibilityResult, RegistryError> {
        let mut result = CompatibilityResult {
            compatible: true,
            conflicts: Vec::new(),
        };

        let blocks: Vec<_> = block_ids.iter()
            .map(|id| self.get_block(id))
            .collect::<Result<_, _>>()?;

        // Check for conflicts
        for (i, block_a) in blocks.iter().enumerate() {
            for block_b in blocks.iter().skip(i + 1) {
                // Check if blocks conflict
                let meta_a = block_a.metadata();
                let meta_b = block_b.metadata();

                // Example: Check if blocks have conflicting guarantees
                // (More sophisticated checking would go here)
            }
        }

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub cycles: Vec<Vec<String>>,
}

impl DependencyGraph {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            cycles: Vec::new(),
        }
    }

    fn add_node(&mut self, node: String) {
        if !self.nodes.contains(&node) {
            self.nodes.push(node);
        }
    }

    fn add_edge(&mut self, from: String, to: String) {
        self.edges.push((from, to));
    }

    fn detect_cycles(&mut self) {
        // Tarjan's algorithm or similar for cycle detection
        // Implementation details omitted for brevity
    }
}

#[derive(Debug, Clone)]
pub struct CompatibilityResult {
    pub compatible: bool,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub block_ids: Vec<String>,
    pub reason: String,
    pub severity: ConflictSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSeverity {
    Error,
    Warning,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Duplicate block: {0}")]
    DuplicateBlock(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Lock error")]
    LockError,
}
```

---

## 5. Constraint System (`core/constraint.rs`)

```rust
use serde::{Deserialize, Serialize};

/// Constraint that a block requires
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    RequiresBlock(String),
    RequiresFeature(String),
    MinimumMemory(usize),
    MinimumDisk(usize),
    ThreadSafe,
    AtomicOperations,
}

/// Guarantee that a block provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guarantee {
    pub guarantee_type: GuaranteeType,
    pub description: String,
    pub level: GuaranteeLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuaranteeType {
    Acid,
    Durability,
    Consistency,
    Isolation,
    Atomicity,
    ThreadSafe,
    Serializable,
    SnapshotIsolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuaranteeLevel {
    Strict,
    BestEffort,
}

/// Context for checking constraints
pub struct ConstraintContext {
    pub blocks: Vec<Arc<dyn Block>>,
    pub configuration: Configuration,
    pub environment: Environment,
}

#[derive(Debug, Clone)]
pub struct Configuration {
    pub memory_limit: Option<usize>,
    pub disk_limit: Option<usize>,
    pub thread_count: usize,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub platform: String,
    pub cpu_cores: usize,
    pub available_memory: usize,
    pub available_disk: usize,
}
```

---

## 6. Metrics System (`core/metrics.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub id: String,
    pub name: String,
    pub metric_type: MetricType,
    pub unit: String,
    pub description: String,
    pub aggregations: Vec<AggregationType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,      // Monotonically increasing
    Gauge,        // Point-in-time value
    Histogram,    // Distribution
    Timing,       // Duration measurements
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationType {
    Sum,
    Avg,
    Min,
    Max,
    P50,  // Median
    P95,
    P99,
}

/// Metrics collector for runtime
pub struct MetricsCollector {
    metrics: Arc<Mutex<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn record(&self, metric_id: &str, value: f64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.entry(metric_id.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    pub fn increment(&self, metric_id: &str) {
        self.record(metric_id, 1.0);
    }

    pub fn get_values(&self, metric_id: &str) -> Vec<f64> {
        let metrics = self.metrics.lock().unwrap();
        metrics.get(metric_id).cloned().unwrap_or_default()
    }

    pub fn aggregate(&self, metric_id: &str, agg_type: AggregationType) -> Option<f64> {
        let values = self.get_values(metric_id);
        if values.is_empty() {
            return None;
        }

        match agg_type {
            AggregationType::Sum => Some(values.iter().sum()),
            AggregationType::Avg => Some(values.iter().sum::<f64>() / values.len() as f64),
            AggregationType::Min => values.iter().cloned().min_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::Max => values.iter().cloned().max_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::P50 => Self::percentile(&values, 0.5),
            AggregationType::P95 => Self::percentile(&values, 0.95),
            AggregationType::P99 => Self::percentile(&values, 0.99),
        }
    }

    fn percentile(values: &[f64], p: f64) -> Option<f64> {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((sorted.len() as f64 - 1.0) * p) as usize;
        sorted.get(idx).cloned()
    }
}

// Logger and storage context stubs
pub struct Logger;
pub struct StorageContext;
```

---

## Implementation Priority

### Phase 1: Core Infrastructure
1. ✅ Define all Rust traits and types
2. Implement `BlockRegistry`
3. Implement parameter validation
4. Implement port validation
5. Set up unit testing with `cargo test`

### Phase 2: First Block
1. Implement `HeapFileBlock` as reference
2. Test with real data
3. Benchmark performance
4. Document patterns

### Testing with Rust
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_block_registration() {
        let registry = BlockRegistry::new();
        // Test implementation
    }
}
```

### Dependencies (Cargo.toml)
```toml
[dependencies]
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tokio = { version = "1.0", features = ["full"] }
uuid = { version = "1.0", features = ["v4", "serde"] }

[dev-dependencies]
criterion = "0.5"  # For benchmarking
```
