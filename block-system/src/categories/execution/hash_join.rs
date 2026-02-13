//! Hash Join Execution Block
//!
//! Implements a classic **build-probe hash join**. The smaller (build) input
//! is loaded into a hash table, then the larger (probe) input is scanned,
//! looking up matches in the hash table.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `build_rows` | Counter | Rows in build side |
//! | `probe_rows` | Counter | Rows in probe side |
//! | `matches` | Counter | Join matches produced |
//! | `hash_buckets` | Gauge | Hash table bucket count |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterType, ParameterUIHint, ParameterValue, ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

fn hash_value(v: &JsonValue) -> u64 {
    let s = v.to_string();
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(1099511628211); }
    h
}

pub struct HashJoinBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    join_column: String,
}

impl HashJoinBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            join_column: "id".into(),
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "hash-join".into(),
            name: "Hash Join".into(),
            category: BlockCategory::Execution,
            description: "Build-probe hash join for equi-join queries".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A hash join is the most efficient join algorithm for equi-join queries \
                           (WHERE a.id = b.id) when one input fits in memory. It works in two \
                           phases: the build phase loads the smaller input into an in-memory hash \
                           table keyed on the join column, and the probe phase streams through the \
                           larger input, looking up each row's join key in the hash table.\n\n\
                           Hash joins are the workhorse of modern query engines. They are chosen \
                           by the optimizer whenever the join predicate is an equality condition \
                           and the smaller input is estimated to fit in the available memory. \
                           Unlike nested loop joins (which are O(n*m)), hash joins are O(n+m) — \
                           a massive improvement for large inputs.\n\n\
                           Think of it like organizing a party with two guest lists. You take the \
                           shorter list and create a lookup table (hash table) by name. Then you \
                           go through the longer list one by one, checking each name against the \
                           lookup table. For each match, you know that guest is on both lists. \
                           Building the lookup table takes one pass, and checking takes one pass — \
                           much faster than comparing every name on list A with every name on list B."
                    .into(),
                algorithm: "Hash Join Algorithm (Build-Probe):\n\
                            \n\
                            // Phase 1: BUILD — create hash table from smaller input\n\
                            FUNCTION build_phase(build_records, join_column):\n  \
                              hash_table = new HashMap\n  \
                              FOR EACH record IN build_records:\n    \
                                key = record[join_column]\n    \
                                h = hash(key)\n    \
                                hash_table[h].append(record)\n  \
                              RETURN hash_table\n\
                            \n\
                            // Phase 2: PROBE — scan larger input and look up matches\n\
                            FUNCTION probe_phase(probe_records, hash_table, join_column):\n  \
                              results = []\n  \
                              FOR EACH probe_rec IN probe_records:\n    \
                                key = probe_rec[join_column]\n    \
                                h = hash(key)\n    \
                                IF h IN hash_table:\n      \
                                  FOR EACH build_rec IN hash_table[h]:\n        \
                                    IF build_rec[join_column] == key:  // verify, not just hash\n          \
                                      combined = merge(build_rec, probe_rec)\n          \
                                      results.append(combined)\n  \
                              RETURN results"
                    .into(),
                complexity: Complexity {
                    time: "O(n + m) average where n = build, m = probe".into(),
                    space: "O(n) for the hash table".into(),
                },
                use_cases: vec![
                    "Equi-join queries (WHERE a.id = b.id)".into(),
                    "When one table is much smaller than the other".into(),
                    "Parallel-friendly join strategy".into(),
                    "Star schema queries joining fact tables with dimension tables".into(),
                    "Subquery materialization (IN subqueries converted to semi-joins)".into(),
                ],
                tradeoffs: vec![
                    "Fastest join for equi-joins but only supports equality predicates".into(),
                    "Build side must fit in memory (or spill to disk)".into(),
                    "No ordering on output".into(),
                    "Hash collisions degrade performance — a poor hash function or highly \
                     skewed data can cause many rows to land in the same bucket".into(),
                    "Spilling to disk (grace hash join) adds significant overhead — the \
                     optimizer tries to pick the smaller input as the build side to avoid this".into(),
                ],
                examples: vec![
                    "PostgreSQL Hash Join — builds a hash table from the inner relation; \
                     supports parallel hash join since version 11".into(),
                    "MySQL hash join (8.0+) — added as an alternative to the block nested \
                     loop join; requires equi-join conditions".into(),
                    "Oracle hash join — the preferred join method for large tables; supports \
                     in-memory and on-disk (grace hash join) variants".into(),
                    "DuckDB and other analytics engines — hash join is the default join \
                     strategy for columnar databases".into(),
                ],
                motivation: "Joining tables is the most common multi-table operation in SQL. \
                             Without hash join, the database would fall back to nested loop join \
                             (O(n*m) — checking every pair of rows) or require both inputs to be \
                             sorted for a merge join. For large tables, nested loop is prohibitively \
                             slow, and sorting both inputs is expensive.\n\n\
                             Hash join solves this by building an O(1)-lookup data structure from \
                             one input, reducing the join cost to O(n+m). This made complex \
                             analytical queries with multiple joins practical, and is one of the \
                             key innovations that enabled modern data warehousing."
                    .into(),
                parameter_guide: HashMap::from([
                    ("join_column".into(), "The column name that must exist in both the build and \
                                            probe inputs. Rows are matched when they have equal \
                                            values in this column. This corresponds to the ON clause \
                                            in SQL (e.g., ON orders.customer_id = customers.id). \
                                            The column should have compatible data types in both \
                                            inputs. High-cardinality columns (like IDs) produce \
                                            fewer hash collisions and better performance. \
                                            Low-cardinality columns (like status or gender) cause \
                                            many collisions, degrading to O(n*m) in the worst case.".into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "sort".into(),
                        comparison: "A sort-merge join sorts both inputs on the join column, then \
                                     merges them in order. Sort-merge is better when inputs are \
                                     already sorted (e.g., from an index) or when the result must \
                                     be in sorted order. Hash join is better for unsorted inputs \
                                     because it avoids the O(n log n) sort cost. Sort-merge also \
                                     handles non-equi joins (theta joins) which hash join cannot.".into(),
                    },
                    Alternative {
                        block_type: "sequential-scan".into(),
                        comparison: "A nested loop join (using sequential scans) checks every row \
                                     pair — O(n*m). It is the simplest join but only practical for \
                                     very small inputs or when an index makes the inner loop O(log n). \
                                     Hash join is dramatically faster for medium-to-large inputs with \
                                     equi-join predicates.".into(),
                    },
                ],
                suggested_questions: vec![
                    "Why does the optimizer choose the smaller table as the build side, \
                     and what happens if it gets the estimate wrong?".into(),
                    "What is a grace hash join, and how does it handle the case when the \
                     build side does not fit in memory?".into(),
                    "How does parallel hash join work, and why is it easier to parallelize \
                     than sort-merge join?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database System Concepts — Chapter 15: Join Algorithms".into(),
                url: None,
                citation: Some("Silberschatz, A. et al. (2019). McGraw-Hill.".into()),
            }],
            icon: "merge".into(),
            color: "#0EA5E9".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![
            Port {
                id: "build".into(), name: "Build Side".into(), port_type: PortType::DataStream,
                direction: PortDirection::Input, required: true, multiple: false,
                description: "Smaller input — loaded into hash table".into(), schema: None,
            },
            Port {
                id: "probe".into(), name: "Probe Side".into(), port_type: PortType::DataStream,
                direction: PortDirection::Input, required: true, multiple: false,
                description: "Larger input — scanned for matches".into(), schema: None,
            },
        ]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "joined".into(), name: "Joined Records".into(), port_type: PortType::DataStream,
            direction: PortDirection::Output, required: false, multiple: true,
            description: "Matched rows from both inputs combined".into(), schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "join_column".into(), name: "Join Column".into(), param_type: ParameterType::String,
            description: "Column to join on (must exist in both inputs)".into(),
            default_value: ParameterValue::String("id".into()), required: true, constraints: None,
            ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition { id: "build_rows".into(), name: "Build Rows".into(), metric_type: MetricType::Counter, unit: "rows".into(), description: "Rows in build side".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "probe_rows".into(), name: "Probe Rows".into(), metric_type: MetricType::Counter, unit: "rows".into(), description: "Rows in probe side".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "matches".into(), name: "Matches".into(), metric_type: MetricType::Counter, unit: "rows".into(), description: "Join matches produced".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "hash_buckets".into(), name: "Hash Buckets".into(), metric_type: MetricType::Gauge, unit: "buckets".into(), description: "Hash table size".into(), aggregations: vec![AggregationType::Max] },
        ]
    }
}

impl Default for HashJoinBlock {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Block for HashJoinBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] { &[] }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(&mut self, params: HashMap<String, ParameterValue>) -> Result<(), BlockError> {
        if let Some(v) = params.get("join_column") { if let Some(s) = v.as_string() { self.join_column = s.to_string(); } }
        Ok(())
    }

    async fn execute(&mut self, context: ExecutionContext) -> Result<ExecutionResult, BlockError> {
        let extract = |key: &str| -> Vec<Record> {
            match context.inputs.get(key).cloned().unwrap_or(PortValue::None) {
                PortValue::Stream(r) => r, PortValue::Batch(r) => r, PortValue::Single(r) => vec![r],
                _ => Vec::new(),
            }
        };
        let build_records = extract("build");
        let probe_records = extract("probe");

        // Build phase: hash table from build side.
        let mut ht: HashMap<u64, Vec<&Record>> = HashMap::new();
        for rec in &build_records {
            let key = rec.data.get(&self.join_column).unwrap_or(&JsonValue::Null);
            let h = hash_value(key);
            ht.entry(h).or_default().push(rec);
        }

        // Probe phase.
        let mut joined = Vec::new();
        for probe_rec in &probe_records {
            let key = probe_rec.data.get(&self.join_column).unwrap_or(&JsonValue::Null);
            let h = hash_value(key);
            if let Some(build_recs) = ht.get(&h) {
                for build_rec in build_recs {
                    // Verify actual key match (not just hash).
                    let bk = build_rec.data.get(&self.join_column).unwrap_or(&JsonValue::Null);
                    if bk == key {
                        // Merge fields from both records.
                        let mut combined = Record::new();
                        for (k, v) in &build_rec.data {
                            let _ = combined.data.insert(format!("build_{}", k), v.clone());
                        }
                        for (k, v) in &probe_rec.data {
                            let _ = combined.data.insert(format!("probe_{}", k), v.clone());
                        }
                        joined.push(combined);
                    }
                }
            }
        }

        let matches = joined.len();
        context.metrics.record("build_rows", build_records.len() as f64);
        context.metrics.record("probe_rows", probe_records.len() as f64);
        context.metrics.record("matches", matches as f64);
        context.metrics.record("hash_buckets", ht.len() as f64);

        let mut outputs = HashMap::new();
        outputs.insert("joined".into(), PortValue::Stream(joined));
        let mut ms = HashMap::new();
        ms.insert("build_rows".into(), build_records.len() as f64);
        ms.insert("probe_rows".into(), probe_records.len() as f64);
        ms.insert("matches".into(), matches as f64);

        Ok(ExecutionResult { outputs, metrics: ms, errors: vec![] })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        let has_build = inputs.get("build").is_some();
        let has_probe = inputs.get("probe").is_some();
        if !has_build && !has_probe { ValidationResult::ok().with_warning("Neither build nor probe connected") }
        else if !has_build { ValidationResult::ok().with_warning("build input not connected") }
        else if !has_probe { ValidationResult::ok().with_warning("probe input not connected") }
        else { ValidationResult::ok() }
    }
    fn get_state(&self) -> BlockState { BlockState::new() }
    fn set_state(&mut self, _: BlockState) -> Result<(), BlockError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_join_basic() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut hj = HashJoinBlock::new();
        hj.join_column = "id".into();

        let build: Vec<Record> = (0..5).map(|i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r.insert("name".into(), format!("b{}", i)).unwrap(); r }).collect();
        let probe: Vec<Record> = (3..8).map(|i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r.insert("val".into(), format!("p{}", i)).unwrap(); r }).collect();

        let mut inputs = HashMap::new();
        inputs.insert("build".into(), PortValue::Stream(build));
        inputs.insert("probe".into(), PortValue::Stream(probe));

        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = hj.execute(ctx).await.unwrap();

        // Overlap: ids 3, 4 → 2 matches.
        assert_eq!(*result.metrics.get("matches").unwrap(), 2.0);
    }

    #[tokio::test]
    async fn test_hash_join_no_matches() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut hj = HashJoinBlock::new();
        hj.join_column = "id".into();

        let build: Vec<Record> = (0..3).map(|i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r }).collect();
        let probe: Vec<Record> = (10..13).map(|i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r }).collect();

        let mut inputs = HashMap::new();
        inputs.insert("build".into(), PortValue::Stream(build));
        inputs.insert("probe".into(), PortValue::Stream(probe));

        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = hj.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("matches").unwrap(), 0.0);
    }

    #[test]
    fn test_metadata() {
        let hj = HashJoinBlock::new();
        assert_eq!(hj.metadata().id, "hash-join");
        assert_eq!(hj.metadata().category, BlockCategory::Execution);
        assert_eq!(hj.inputs().len(), 2);
    }
}
