//! B-tree Index Block
//!
//! A classic B-tree index that maps key values to [`TupleId`]s. It supports
//! point lookups and ordered range scans, making it the workhorse index
//! structure in virtually all relational databases.
//!
//! ## How it works
//!
//! The tree consists of **internal nodes** (which hold keys and child pointers)
//! and **leaf nodes** (which hold keys and TupleId values). A configurable
//! **fanout** (order) controls how many keys fit per node, which directly
//! affects tree depth and therefore lookup speed.
//!
//! Leaf nodes are linked via `next_leaf` pointers so range scans can walk the
//! leaf chain without revisiting internal nodes.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `tree_depth` | Gauge | Current depth of the tree |
//! | `total_keys` | Gauge | Number of indexed keys |
//! | `lookups` | Counter | Point lookups performed |
//! | `range_scans` | Counter | Range scans performed |
//! | `splits` | Counter | Node splits during insert |
//! | `comparisons` | Counter | Key comparisons made |

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
// Internal B-tree model
// ---------------------------------------------------------------------------

/// An entry stored in a leaf node.
#[derive(Debug, Clone)]
struct LeafEntry {
    key: JsonValue,
    tuple_id: TupleId,
}

/// A B-tree node (either internal or leaf).
#[derive(Debug, Clone)]
enum BTreeNode {
    Internal {
        keys: Vec<JsonValue>,
        children: Vec<usize>, // indices into the nodes Vec
    },
    Leaf {
        entries: Vec<LeafEntry>,
        next_leaf: Option<usize>, // linked-list for range scans
    },
}

/// Compare two JSON values for ordering.
/// Numbers are compared numerically, strings lexicographically.
fn cmp_json(a: &JsonValue, b: &JsonValue) -> std::cmp::Ordering {
    match (a, b) {
        (JsonValue::Number(na), JsonValue::Number(nb)) => {
            let fa = na.as_f64().unwrap_or(0.0);
            let fb = nb.as_f64().unwrap_or(0.0);
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        }
        (JsonValue::String(sa), JsonValue::String(sb)) => sa.cmp(sb),
        // Fall back to string representation for mixed types.
        _ => a.to_string().cmp(&b.to_string()),
    }
}

// ---------------------------------------------------------------------------
// BTreeIndexBlock
// ---------------------------------------------------------------------------

pub struct BTreeIndexBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    fanout: usize,
    key_column: String,
    unique: bool,

    // Internal state
    nodes: Vec<BTreeNode>,
    root: usize,
    total_keys: usize,
    split_count: usize,
    comparison_count: usize,
}

impl BTreeIndexBlock {
    pub fn new() -> Self {
        let mut block = Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            fanout: 128,
            key_column: "id".into(),
            unique: false,
            nodes: Vec::new(),
            root: 0,
            total_keys: 0,
            split_count: 0,
            comparison_count: 0,
        };
        // Start with an empty leaf as root.
        block.nodes.push(BTreeNode::Leaf {
            entries: Vec::new(),
            next_leaf: None,
        });
        block
    }

    // -- Metadata builders ---------------------------------------------------

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "btree-index".into(),
            name: "B-tree Index".into(),
            category: BlockCategory::Index,
            description: "Balanced tree index for point lookups and ordered range scans".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A B-tree keeps keys sorted across a balanced, multi-way tree. \
                           Lookups, inserts, and deletes are all O(log n) because the tree \
                           depth grows logarithmically with the number of keys."
                    .into(),
                algorithm: "Insert: walk from root to correct leaf, insert key. If the leaf \
                            overflows (> fanout entries), split it and push the median key \
                            up to the parent. Repeat splits upward as needed. Lookup: binary \
                            search at each level. Range scan: find start leaf, walk the \
                            linked leaf chain."
                    .into(),
                complexity: Complexity {
                    time: "O(log_f n) lookup/insert where f = fanout".into(),
                    space: "O(n) — one entry per indexed key".into(),
                },
                use_cases: vec![
                    "Primary key indexes".into(),
                    "Range queries (BETWEEN, ORDER BY)".into(),
                    "Unique constraint enforcement".into(),
                ],
                tradeoffs: vec![
                    "Higher fanout → shallower tree but more comparisons per node".into(),
                    "Writes cause node splits which are expensive".into(),
                    "Not ideal for high-cardinality write-heavy workloads (consider LSM)".into(),
                ],
                examples: vec![
                    "PostgreSQL B-tree indexes (default CREATE INDEX)".into(),
                    "InnoDB secondary indexes".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Paper,
                title: "Organization and Maintenance of Large Ordered Indexes".into(),
                url: None,
                citation: Some(
                    "Bayer, R. & McCreight, E. (1972). Acta Informatica, 1(3), 173–189.".into(),
                ),
            }],
            icon: "git-branch".into(),
            color: "#10B981".into(),
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
            description: "Results of point lookups or range scans".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "fanout".into(),
                name: "Fanout (Order)".into(),
                param_type: ParameterType::Number,
                description: "Maximum number of keys per node".into(),
                default_value: ParameterValue::Integer(128),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(3.0).with_max(1024.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(1.0)
                        .with_help_text("Higher fanout = shallower tree, more comparisons per node".into()),
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
            Parameter {
                id: "unique".into(),
                name: "Unique".into(),
                param_type: ParameterType::Boolean,
                description: "Reject duplicate key values".into(),
                default_value: ParameterValue::Boolean(false),
                required: false,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Checkbox)),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "tree_depth".into(),
                name: "Tree Depth".into(),
                metric_type: MetricType::Gauge,
                unit: "levels".into(),
                description: "Current depth of the B-tree".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "total_keys".into(),
                name: "Total Keys".into(),
                metric_type: MetricType::Gauge,
                unit: "keys".into(),
                description: "Number of indexed keys".into(),
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
                id: "range_scans".into(),
                name: "Range Scans".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Range scans performed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "splits".into(),
                name: "Node Splits".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Node splits during inserts".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "comparisons".into(),
                name: "Comparisons".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Key comparisons made".into(),
                aggregations: vec![AggregationType::Sum],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Calculate tree depth from root.
    pub fn depth(&self) -> usize {
        let mut d = 1;
        let mut idx = self.root;
        loop {
            match &self.nodes[idx] {
                BTreeNode::Internal { children, .. } => {
                    d += 1;
                    idx = children[0];
                }
                BTreeNode::Leaf { .. } => return d,
            }
        }
    }

    /// Insert a key→TupleId mapping.
    ///
    /// Returns `Err` if `unique` is true and the key already exists.
    pub fn insert_key(
        &mut self,
        key: JsonValue,
        tuple_id: TupleId,
    ) -> Result<(), String> {
        if self.unique && self.lookup(&key).is_some() {
            return Err(format!("Duplicate key: {}", key));
        }

        let result = self.insert_recursive(self.root, key, tuple_id);

        if let Some((median, new_child)) = result {
            // Root was split — create a new root.
            let old_root = self.root;
            let new_root = BTreeNode::Internal {
                keys: vec![median],
                children: vec![old_root, new_child],
            };
            let new_root_idx = self.nodes.len();
            self.nodes.push(new_root);
            self.root = new_root_idx;
            self.split_count += 1;
        }

        self.total_keys += 1;
        Ok(())
    }

    /// Recursively insert into the subtree rooted at `node_idx`.
    /// Returns `Some((median_key, new_node_idx))` if the node was split.
    fn insert_recursive(
        &mut self,
        node_idx: usize,
        key: JsonValue,
        tuple_id: TupleId,
    ) -> Option<(JsonValue, usize)> {
        match self.nodes[node_idx].clone() {
            BTreeNode::Leaf { mut entries, next_leaf } => {
                // Find position via binary search.
                let pos = entries
                    .binary_search_by(|e| {
                        self.comparison_count += 1;
                        cmp_json(&e.key, &key)
                    })
                    .unwrap_or_else(|p| p);

                entries.insert(pos, LeafEntry { key, tuple_id });

                if entries.len() > self.fanout {
                    // Split the leaf.
                    let mid = entries.len() / 2;
                    let right_entries = entries.split_off(mid);
                    let median = right_entries[0].key.clone();

                    let new_leaf_idx = self.nodes.len();

                    // Left leaf keeps entries[..mid], points to new right leaf.
                    self.nodes[node_idx] = BTreeNode::Leaf {
                        entries,
                        next_leaf: Some(new_leaf_idx),
                    };
                    // Right leaf gets entries[mid..], inherits old next_leaf.
                    self.nodes.push(BTreeNode::Leaf {
                        entries: right_entries,
                        next_leaf,
                    });

                    self.split_count += 1;
                    Some((median, new_leaf_idx))
                } else {
                    self.nodes[node_idx] = BTreeNode::Leaf { entries, next_leaf };
                    None
                }
            }
            BTreeNode::Internal { keys, children } => {
                // Find which child to descend into.
                let mut child_pos = keys.len();
                for (i, k) in keys.iter().enumerate() {
                    self.comparison_count += 1;
                    if cmp_json(&key, k) == std::cmp::Ordering::Less {
                        child_pos = i;
                        break;
                    }
                }

                let child_idx = children[child_pos];
                let split_result = self.insert_recursive(child_idx, key, tuple_id);

                if let Some((median, new_child_idx)) = split_result {
                    let mut keys = self.internal_keys(node_idx);
                    let children_ref = self.internal_children_mut(node_idx);
                    // Insert median and new child pointer.
                    keys.insert(child_pos, median);
                    children_ref.insert(child_pos + 1, new_child_idx);
                    let children_new = children_ref.clone();

                    if keys.len() > self.fanout {
                        // Split the internal node.
                        let mid = keys.len() / 2;
                        let up_key = keys[mid].clone();

                        let right_keys: Vec<_> = keys.drain(mid + 1..).collect();
                        keys.truncate(mid);
                        // children_new has keys.len()+1 entries before drain.
                        // After splitting keys at mid, left gets keys[0..mid], right gets keys[mid+1..].
                        let left_children: Vec<_> = children_new[..mid + 1].to_vec();
                        let right_children: Vec<_> = children_new[mid + 1..].to_vec();

                        let new_internal_idx = self.nodes.len();
                        self.nodes[node_idx] = BTreeNode::Internal {
                            keys,
                            children: left_children,
                        };
                        self.nodes.push(BTreeNode::Internal {
                            keys: right_keys,
                            children: right_children,
                        });

                        self.split_count += 1;
                        Some((up_key, new_internal_idx))
                    } else {
                        self.nodes[node_idx] = BTreeNode::Internal {
                            keys,
                            children: children_new,
                        };
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    fn internal_keys(&self, idx: usize) -> Vec<JsonValue> {
        match &self.nodes[idx] {
            BTreeNode::Internal { keys, .. } => keys.clone(),
            _ => vec![],
        }
    }

    fn internal_children_mut(&mut self, idx: usize) -> &mut Vec<usize> {
        match &mut self.nodes[idx] {
            BTreeNode::Internal { children, .. } => children,
            _ => panic!("Expected internal node"),
        }
    }

    /// Point lookup — returns the first matching TupleId.
    pub fn lookup(&mut self, key: &JsonValue) -> Option<TupleId> {
        let mut idx = self.root;
        loop {
            match &self.nodes[idx] {
                BTreeNode::Internal { keys, children } => {
                    let mut child_pos = keys.len();
                    for (i, k) in keys.iter().enumerate() {
                        self.comparison_count += 1;
                        if cmp_json(key, k) == std::cmp::Ordering::Less {
                            child_pos = i;
                            break;
                        }
                    }
                    idx = children[child_pos];
                }
                BTreeNode::Leaf { entries, .. } => {
                    for entry in entries {
                        self.comparison_count += 1;
                        if cmp_json(&entry.key, key) == std::cmp::Ordering::Equal {
                            return Some(entry.tuple_id);
                        }
                    }
                    return None;
                }
            }
        }
    }

    /// Range scan — returns all entries where start <= key <= end, in order.
    pub fn range_scan(
        &mut self,
        start: &JsonValue,
        end: &JsonValue,
    ) -> Vec<(JsonValue, TupleId)> {
        let mut results = Vec::new();

        // Walk to the leaf that might contain `start`.
        let mut idx = self.root;
        loop {
            match &self.nodes[idx] {
                BTreeNode::Internal { keys, children } => {
                    let mut child_pos = keys.len();
                    for (i, k) in keys.iter().enumerate() {
                        self.comparison_count += 1;
                        if cmp_json(start, k) == std::cmp::Ordering::Less {
                            child_pos = i;
                            break;
                        }
                    }
                    idx = children[child_pos];
                }
                BTreeNode::Leaf { .. } => break,
            }
        }

        // Walk the leaf chain collecting matching entries.
        loop {
            let (entries, next) = match &self.nodes[idx] {
                BTreeNode::Leaf { entries, next_leaf } => (entries.clone(), *next_leaf),
                _ => break,
            };

            for entry in &entries {
                self.comparison_count += 1;
                if cmp_json(&entry.key, start) == std::cmp::Ordering::Less {
                    continue;
                }
                if cmp_json(&entry.key, end) == std::cmp::Ordering::Greater {
                    return results;
                }
                results.push((entry.key.clone(), entry.tuple_id));
            }

            match next {
                Some(next_idx) => idx = next_idx,
                None => break,
            }
        }

        results
    }

    pub fn key_count(&self) -> usize {
        self.total_keys
    }
}

impl Default for BTreeIndexBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for BTreeIndexBlock {
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
                "Keys are always maintained in sorted order",
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
        if let Some(val) = params.get("fanout") {
            let f = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("fanout must be an integer".into()))?
                as usize;
            if f < 3 || f > 1024 {
                return Err(BlockError::InvalidParameter(
                    "fanout must be between 3 and 1024".into(),
                ));
            }
            self.fanout = f;
        }
        if let Some(val) = params.get("key_column") {
            self.key_column = val
                .as_string()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("key_column must be a string".into())
                })?
                .to_string();
        }
        if let Some(val) = params.get("unique") {
            self.unique = val.as_bool().ok_or_else(|| {
                BlockError::InvalidParameter("unique must be a boolean".into())
            })?;
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

        let mut errors = Vec::new();

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

            let tid = TupleId::new(page_id, slot_id);

            if let Err(e) = self.insert_key(key, tid) {
                errors.push(BlockError::ExecutionError(e));
            }
        }

        // Record metrics.
        context.metrics.record("tree_depth", self.depth() as f64);
        context
            .metrics
            .record("total_keys", self.total_keys as f64);
        context
            .metrics
            .record("splits", self.split_count as f64);
        context
            .metrics
            .record("comparisons", self.comparison_count as f64);

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("tree_depth".into(), self.depth() as f64);
        metrics_summary.insert("total_keys".into(), self.total_keys as f64);
        metrics_summary.insert("splits".into(), self.split_count as f64);

        Ok(ExecutionResult {
            outputs: HashMap::new(),
            metrics: metrics_summary,
            errors,
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
        let _ = state.insert("fanout".into(), self.fanout);
        let _ = state.insert("key_column".into(), self.key_column.clone());
        let _ = state.insert("unique".into(), self.unique);
        let _ = state.insert("total_keys".into(), self.total_keys);
        let _ = state.insert("depth".into(), self.depth());
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(f)) = state.get::<usize>("fanout") {
            self.fanout = f;
        }
        if let Ok(Some(kc)) = state.get::<String>("key_column") {
            self.key_column = kc;
        }
        if let Ok(Some(u)) = state.get::<bool>("unique") {
            self.unique = u;
        }
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
        let mut tree = BTreeIndexBlock::new();
        tree.fanout = 4; // Small fanout to trigger splits easily.

        for i in 0..20 {
            tree.insert_key(json!(i), TupleId::new(0, i as usize))
                .unwrap();
        }

        assert_eq!(tree.key_count(), 20);

        // Point lookup for each key.
        for i in 0..20 {
            let result = tree.lookup(&json!(i));
            assert!(result.is_some(), "Key {} not found", i);
            assert_eq!(result.unwrap().slot_id, i as usize);
        }

        // Non-existent key.
        assert!(tree.lookup(&json!(999)).is_none());
    }

    #[test]
    fn test_depth_grows_logarithmically() {
        let mut tree = BTreeIndexBlock::new();
        tree.fanout = 4;

        for i in 0..10_000 {
            tree.insert_key(json!(i), TupleId::new(0, i as usize))
                .unwrap();
        }

        let depth = tree.depth();
        // With fanout 4 and 10K keys: depth ≤ log_4(10000) ≈ 6.6 → expect ≤ 8
        assert!(
            depth <= 10,
            "Depth {} is too large for 10K keys with fanout 4",
            depth
        );
        assert!(depth >= 2, "Tree should have at least depth 2");
    }

    #[test]
    fn test_range_scan_returns_ordered() {
        let mut tree = BTreeIndexBlock::new();
        tree.fanout = 4;

        // Insert keys 0..100 in random-ish order.
        let keys: Vec<i64> = (0..100).collect();
        for &k in keys.iter().rev() {
            tree.insert_key(json!(k), TupleId::new(0, k as usize))
                .unwrap();
        }

        let results = tree.range_scan(&json!(20), &json!(30));
        assert_eq!(results.len(), 11, "Range [20,30] should have 11 entries");

        // Verify ordering.
        for i in 0..results.len() - 1 {
            assert!(
                cmp_json(&results[i].0, &results[i + 1].0) != std::cmp::Ordering::Greater,
                "Range scan results should be ordered"
            );
        }
    }

    #[test]
    fn test_unique_constraint() {
        let mut tree = BTreeIndexBlock::new();
        tree.unique = true;

        tree.insert_key(json!(1), TupleId::new(0, 0)).unwrap();
        let dup = tree.insert_key(json!(1), TupleId::new(0, 1));
        assert!(dup.is_err(), "Duplicate key should be rejected");
    }

    #[test]
    fn test_string_keys() {
        let mut tree = BTreeIndexBlock::new();
        tree.fanout = 4;

        let names = vec!["Alice", "Bob", "Charlie", "Diana", "Eve"];
        for (i, name) in names.iter().enumerate() {
            tree.insert_key(json!(name), TupleId::new(0, i)).unwrap();
        }

        assert!(tree.lookup(&json!("Charlie")).is_some());
        assert!(tree.lookup(&json!("Frank")).is_none());

        let range = tree.range_scan(&json!("B"), &json!("D"));
        assert!(
            range.len() >= 2,
            "Should find Bob and Charlie in [B, D]"
        );
    }

    #[test]
    fn test_splits_counted() {
        let mut tree = BTreeIndexBlock::new();
        tree.fanout = 3;

        for i in 0..20 {
            tree.insert_key(json!(i), TupleId::new(0, i as usize))
                .unwrap();
        }

        assert!(
            tree.split_count > 0,
            "Inserting 20 keys with fanout 3 should cause splits"
        );
    }

    #[test]
    fn test_metadata() {
        let tree = BTreeIndexBlock::new();
        assert_eq!(tree.metadata().id, "btree-index");
        assert_eq!(tree.metadata().category, BlockCategory::Index);
        assert_eq!(tree.inputs().len(), 1);
        assert_eq!(tree.outputs().len(), 1);
        assert_eq!(tree.parameters().len(), 3);
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut tree = BTreeIndexBlock::new();
        tree.fanout = 8;

        let mut records: Vec<Record> = Vec::new();
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), i as i64).unwrap();
            r.insert("_page_id".into(), 0usize).unwrap();
            r.insert("_slot_id".into(), i as usize).unwrap();
            records.push(r);
        }

        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = tree.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("total_keys").unwrap(), 50.0);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_initialize_with_params() {
        let mut tree = BTreeIndexBlock::new();
        let mut params = HashMap::new();
        params.insert("fanout".into(), ParameterValue::Integer(16));
        params.insert("key_column".into(), ParameterValue::String("name".into()));
        params.insert("unique".into(), ParameterValue::Boolean(true));

        tree.initialize(params).await.unwrap();
        assert_eq!(tree.fanout, 16);
        assert_eq!(tree.key_column, "name");
        assert!(tree.unique);
    }
}
