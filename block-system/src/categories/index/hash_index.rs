//! Hash Index Block
//!
//! A hash-based index that maps key values to [`TupleId`]s using a hash table
//! with **bucket chaining** for collision resolution. Provides O(1) average-case
//! point lookups but does not support range scans.
//!
//! ## How it works
//!
//! Keys are hashed to a bucket number. Each bucket is a chain (Vec) of entries.
//! When the **load factor** (entries / buckets) exceeds a threshold, the table
//! is **rehashed** — the bucket count doubles and all entries are redistributed.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `total_keys` | Gauge | Number of indexed keys |
//! | `bucket_count` | Gauge | Current number of buckets |
//! | `load_factor` | Gauge | entries / buckets |
//! | `lookups` | Counter | Point lookups performed |
//! | `collisions` | Counter | Inserts that hit an occupied bucket |
//! | `rehashes` | Counter | Table resizes performed |
//! | `max_chain_len` | Gauge | Longest bucket chain |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::categories::TupleId;
use crate::core::block::{
    Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee, GuaranteeType};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// Internal hash table model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct HashEntry {
    key: JsonValue,
    tuple_id: TupleId,
}

/// A bucket is a chain of entries.
type Bucket = Vec<HashEntry>;

/// Simple FNV-1a hash for JSON values.
fn hash_json(value: &JsonValue) -> u64 {
    let s = value.to_string();
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

// ---------------------------------------------------------------------------
// HashIndexBlock
// ---------------------------------------------------------------------------

pub struct HashIndexBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    initial_buckets: usize,
    max_load_factor: f64,
    key_column: String,

    // Internal state
    buckets: Vec<Bucket>,
    total_keys: usize,
    collision_count: usize,
    rehash_count: usize,
    lookup_count: usize,
}

impl HashIndexBlock {
    pub fn new() -> Self {
        let initial = 64;
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            initial_buckets: initial,
            max_load_factor: 0.75,
            key_column: "id".into(),
            buckets: vec![Vec::new(); initial],
            total_keys: 0,
            collision_count: 0,
            rehash_count: 0,
            lookup_count: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "hash-index".into(),
            name: "Hash Index".into(),
            category: BlockCategory::Index,
            description: "Hash-based index for O(1) point lookups with bucket chaining".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A hash index provides constant-time average-case point lookups by \
                           hashing keys to bucket positions. Collisions are resolved via chaining \
                           (each bucket holds a list of entries). Unlike B-trees, hash indexes \
                           cannot serve range queries."
                    .into(),
                algorithm: "Insert: hash key → bucket index, append to chain. If load factor \
                            exceeds threshold, double the bucket count and rehash all entries. \
                            Lookup: hash key → bucket index, scan chain for match."
                    .into(),
                complexity: Complexity {
                    time: "O(1) average lookup/insert, O(n) worst case with many collisions"
                        .into(),
                    space: "O(n) entries + O(b) buckets".into(),
                },
                use_cases: vec![
                    "Equality lookups (WHERE id = ?)".into(),
                    "Join operations (hash join build side)".into(),
                    "Primary key constraint enforcement".into(),
                ],
                tradeoffs: vec![
                    "O(1) lookups but no range scan support".into(),
                    "Rehashing is expensive (O(n)) but amortized over inserts".into(),
                    "Performance degrades if hash function has poor distribution".into(),
                ],
                examples: vec![
                    "PostgreSQL hash indexes".into(),
                    "In-memory hash tables for hash joins".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 7: Hash Indexes".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "hash".into(),
            color: "#F59E0B".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(),
            name: "Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Records to index (must contain key_column and _page_id/_slot_id)".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "lookup_results".into(),
            name: "Lookup Results".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Results of point lookups".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "initial_buckets".into(),
                name: "Initial Buckets".into(),
                param_type: ParameterType::Number,
                description: "Starting number of hash buckets".into(),
                default_value: ParameterValue::Integer(64),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(4.0).with_max(65536.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(1.0)
                        .with_help_text("Power of 2 recommended".into()),
                ),
            },
            Parameter {
                id: "max_load_factor".into(),
                name: "Max Load Factor".into(),
                param_type: ParameterType::Number,
                description: "Rehash when entries/buckets exceeds this threshold".into(),
                default_value: ParameterValue::Number(0.75),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(0.1).with_max(2.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(0.05)
                        .with_help_text("Lower = less collisions but more memory".into()),
                ),
            },
            Parameter {
                id: "key_column".into(),
                name: "Key Column".into(),
                param_type: ParameterType::String,
                description: "Name of the column to index".into(),
                default_value: ParameterValue::String("id".into()),
                required: true,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "total_keys".into(),
                name: "Total Keys".into(),
                metric_type: MetricType::Gauge,
                unit: "keys".into(),
                description: "Number of indexed keys".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "bucket_count".into(),
                name: "Bucket Count".into(),
                metric_type: MetricType::Gauge,
                unit: "buckets".into(),
                description: "Current number of hash buckets".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "load_factor".into(),
                name: "Load Factor".into(),
                metric_type: MetricType::Gauge,
                unit: "ratio".into(),
                description: "entries / buckets".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "lookups".into(),
                name: "Lookups".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Point lookups performed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "collisions".into(),
                name: "Collisions".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Inserts that hit a non-empty bucket".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "rehashes".into(),
                name: "Rehashes".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Table resize operations".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "max_chain_len".into(),
                name: "Max Chain Length".into(),
                metric_type: MetricType::Gauge,
                unit: "entries".into(),
                description: "Longest bucket chain".into(),
                aggregations: vec![AggregationType::Max],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    fn bucket_index(&self, key: &JsonValue) -> usize {
        (hash_json(key) as usize) % self.buckets.len()
    }

    /// Insert a key→TupleId mapping.
    pub fn insert_key(&mut self, key: JsonValue, tuple_id: TupleId) {
        let idx = self.bucket_index(&key);
        if !self.buckets[idx].is_empty() {
            self.collision_count += 1;
        }
        self.buckets[idx].push(HashEntry { key, tuple_id });
        self.total_keys += 1;

        // Check load factor.
        if self.load_factor() > self.max_load_factor {
            self.rehash();
        }
    }

    /// Point lookup — returns the first matching TupleId.
    pub fn lookup(&mut self, key: &JsonValue) -> Option<TupleId> {
        self.lookup_count += 1;
        let idx = self.bucket_index(key);
        for entry in &self.buckets[idx] {
            if entry.key == *key {
                return Some(entry.tuple_id);
            }
        }
        None
    }

    /// Current load factor.
    pub fn load_factor(&self) -> f64 {
        self.total_keys as f64 / self.buckets.len() as f64
    }

    /// Maximum chain length across all buckets.
    pub fn max_chain_length(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).max().unwrap_or(0)
    }

    /// Double the bucket count and redistribute all entries.
    fn rehash(&mut self) {
        let new_size = self.buckets.len() * 2;
        let old_buckets = std::mem::replace(&mut self.buckets, vec![Vec::new(); new_size]);

        for bucket in old_buckets {
            for entry in bucket {
                let idx = (hash_json(&entry.key) as usize) % new_size;
                self.buckets[idx].push(entry);
            }
        }

        self.rehash_count += 1;
    }
}

impl Default for HashIndexBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for HashIndexBlock {
    fn metadata(&self) -> &BlockMetadata {
        &self.metadata
    }

    fn inputs(&self) -> &[Port] {
        &self.input_ports
    }

    fn outputs(&self) -> &[Port] {
        &self.output_ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.params
    }

    fn requires(&self) -> &[Constraint] {
        &[]
    }

    fn guarantees(&self) -> &[Guarantee] {
        static GUARANTEES: std::sync::LazyLock<Vec<Guarantee>> = std::sync::LazyLock::new(|| {
            vec![Guarantee::strict(
                GuaranteeType::Consistency,
                "O(1) average-case point lookups",
            )]
        });
        &GUARANTEES
    }

    fn metrics(&self) -> &[MetricDefinition] {
        &self.metric_defs
    }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("initial_buckets") {
            let v = val.as_integer().ok_or_else(|| {
                BlockError::InvalidParameter("initial_buckets must be an integer".into())
            })? as usize;
            if v < 4 || v > 65536 {
                return Err(BlockError::InvalidParameter(
                    "initial_buckets must be between 4 and 65536".into(),
                ));
            }
            self.initial_buckets = v;
            self.buckets = vec![Vec::new(); v];
        }
        if let Some(val) = params.get("max_load_factor") {
            self.max_load_factor = val.as_number().ok_or_else(|| {
                BlockError::InvalidParameter("max_load_factor must be a number".into())
            })?;
            if !(0.1..=2.0).contains(&self.max_load_factor) {
                return Err(BlockError::InvalidParameter(
                    "max_load_factor must be between 0.1 and 2.0".into(),
                ));
            }
        }
        if let Some(val) = params.get("key_column") {
            self.key_column = val
                .as_string()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("key_column must be a string".into())
                })?
                .to_string();
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context
            .inputs
            .get("records")
            .cloned()
            .unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => {
                return Err(BlockError::InvalidInput(
                    "Expected DataStream, Batch, or Single".into(),
                ))
            }
        };

        for record in &records {
            let key = record
                .data
                .get(&self.key_column)
                .cloned()
                .unwrap_or(JsonValue::Null);

            let page_id = record
                .get::<usize>("_page_id")
                .ok()
                .flatten()
                .unwrap_or(0);
            let slot_id = record
                .get::<usize>("_slot_id")
                .ok()
                .flatten()
                .unwrap_or(0);

            self.insert_key(key, TupleId::new(page_id, slot_id));
        }

        context
            .metrics
            .record("total_keys", self.total_keys as f64);
        context
            .metrics
            .record("bucket_count", self.buckets.len() as f64);
        context.metrics.record("load_factor", self.load_factor());
        context
            .metrics
            .record("collisions", self.collision_count as f64);
        context
            .metrics
            .record("rehashes", self.rehash_count as f64);
        context
            .metrics
            .record("max_chain_len", self.max_chain_length() as f64);

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("total_keys".into(), self.total_keys as f64);
        metrics_summary.insert("bucket_count".into(), self.buckets.len() as f64);
        metrics_summary.insert("load_factor".into(), self.load_factor());
        metrics_summary.insert("collisions".into(), self.collision_count as f64);
        metrics_summary.insert("rehashes".into(), self.rehash_count as f64);

        Ok(ExecutionResult {
            outputs: HashMap::new(),
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("records") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => {
                    ValidationResult::ok()
                }
                PortValue::None => {
                    ValidationResult::ok().with_warning("No records to index")
                }
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("total_keys".into(), self.total_keys);
        let _ = state.insert("bucket_count".into(), self.buckets.len());
        let _ = state.insert("load_factor".into(), self.load_factor());
        state
    }

    fn set_state(&mut self, _state: BlockState) -> Result<(), BlockError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_insert_and_lookup() {
        let mut idx = HashIndexBlock::new();

        for i in 0..100 {
            idx.insert_key(json!(i), TupleId::new(0, i as usize));
        }

        assert_eq!(idx.total_keys, 100);

        for i in 0..100 {
            let result = idx.lookup(&json!(i));
            assert!(result.is_some(), "Key {} not found", i);
            assert_eq!(result.unwrap().slot_id, i as usize);
        }

        assert!(idx.lookup(&json!(999)).is_none());
    }

    #[test]
    fn test_rehash_on_load() {
        let mut idx = HashIndexBlock::new();
        idx.buckets = vec![Vec::new(); 8]; // Small starting size
        idx.initial_buckets = 8;
        idx.max_load_factor = 0.75;

        // 8 * 0.75 = 6 entries before rehash
        for i in 0..20 {
            idx.insert_key(json!(i), TupleId::new(0, i as usize));
        }

        assert!(idx.rehash_count > 0, "Should have rehashed");
        assert!(idx.buckets.len() > 8, "Bucket count should have grown");

        // All keys should still be findable.
        for i in 0..20 {
            assert!(idx.lookup(&json!(i)).is_some(), "Key {} missing after rehash", i);
        }
    }

    #[test]
    fn test_collision_counting() {
        let mut idx = HashIndexBlock::new();
        idx.buckets = vec![Vec::new(); 1]; // Force all into one bucket
        idx.max_load_factor = 100.0; // Prevent rehash

        idx.insert_key(json!("a"), TupleId::new(0, 0));
        idx.insert_key(json!("b"), TupleId::new(0, 1));
        idx.insert_key(json!("c"), TupleId::new(0, 2));

        assert_eq!(idx.collision_count, 2, "2nd and 3rd inserts should be collisions");
        assert_eq!(idx.max_chain_length(), 3);
    }

    #[test]
    fn test_load_factor() {
        let mut idx = HashIndexBlock::new();
        idx.buckets = vec![Vec::new(); 100];
        idx.max_load_factor = 10.0; // Prevent rehash

        for i in 0..50 {
            idx.insert_key(json!(i), TupleId::new(0, i as usize));
        }

        let lf = idx.load_factor();
        assert!((lf - 0.5).abs() < 0.01, "Load factor should be 0.5, got {}", lf);
    }

    #[test]
    fn test_string_keys() {
        let mut idx = HashIndexBlock::new();

        idx.insert_key(json!("alice"), TupleId::new(0, 0));
        idx.insert_key(json!("bob"), TupleId::new(0, 1));
        idx.insert_key(json!("charlie"), TupleId::new(0, 2));

        assert!(idx.lookup(&json!("bob")).is_some());
        assert!(idx.lookup(&json!("dave")).is_none());
    }

    #[test]
    fn test_metadata() {
        let idx = HashIndexBlock::new();
        assert_eq!(idx.metadata().id, "hash-index");
        assert_eq!(idx.metadata().category, BlockCategory::Index);
        assert_eq!(idx.inputs().len(), 1);
        assert_eq!(idx.outputs().len(), 1);
        assert_eq!(idx.parameters().len(), 3);
    }

    #[tokio::test]
    async fn test_initialize_with_params() {
        let mut idx = HashIndexBlock::new();
        let mut params = HashMap::new();
        params.insert("initial_buckets".into(), ParameterValue::Integer(128));
        params.insert("max_load_factor".into(), ParameterValue::Number(0.5));
        params.insert("key_column".into(), ParameterValue::String("name".into()));

        idx.initialize(params).await.unwrap();
        assert_eq!(idx.buckets.len(), 128);
        assert!((idx.max_load_factor - 0.5).abs() < f64::EPSILON);
        assert_eq!(idx.key_column, "name");
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut idx = HashIndexBlock::new();

        let records: Vec<Record> = (0..50)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
                r.insert("_page_id".into(), 0usize).unwrap();
                r.insert("_slot_id".into(), i as usize).unwrap();
                r
            })
            .collect();

        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = idx.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("total_keys").unwrap(), 50.0);
        assert!(result.errors.is_empty());
    }
}
