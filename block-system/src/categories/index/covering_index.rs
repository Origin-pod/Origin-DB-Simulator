//! Covering Index Block
//!
//! A **covering index** stores not just keys and TupleIds, but also copies of
//! additional columns. When a query only needs columns that are in the index,
//! it can be answered entirely from the index without touching the base table
//! — an "index-only scan."
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `total_entries` | Gauge | Indexed entries |
//! | `lookups` | Counter | Index lookups |
//! | `index_only_scans` | Counter | Scans answered from index alone |
//! | `table_lookups_avoided` | Counter | Table accesses saved |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee, GuaranteeType};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterType, ParameterUIHint, ParameterValue, ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

/// An index entry that stores the key, covered column values, and original record.
#[derive(Debug, Clone)]
struct CoveringEntry {
    /// The indexed key value
    key: JsonValue,
    /// Copies of covered (included) columns
    covered_values: HashMap<String, JsonValue>,
}

pub struct CoveringIndexBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    key_column: String,
    /// Columns included in the index (besides the key)
    included_columns: Vec<String>,

    /// Sorted index: key_string → Vec<CoveringEntry>
    index: BTreeMap<String, Vec<CoveringEntry>>,
    lookups: usize,
    index_only_scans: usize,
    table_lookups_avoided: usize,
}

impl CoveringIndexBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            key_column: "id".into(),
            included_columns: Vec::new(),
            index: BTreeMap::new(),
            lookups: 0,
            index_only_scans: 0,
            table_lookups_avoided: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "covering-index".into(),
            name: "Covering Index".into(),
            category: BlockCategory::Index,
            description: "Index with included columns for index-only scans".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A covering index is an index that stores additional column values \
                           alongside the index key, enabling the database to answer certain queries \
                           entirely from the index without ever touching the base table. This is \
                           called an 'index-only scan' and it eliminates the random I/O cost of \
                           fetching rows from the heap.\n\n\
                           In a typical B-tree index, a lookup returns a TupleId (page + slot), \
                           and the database must then read that page from the heap to get the \
                           actual row data. For a query like SELECT name, email FROM users WHERE \
                           id = 42, a plain index on 'id' finds the TupleId, then reads the heap \
                           page. A covering index on 'id' that INCLUDEs 'name' and 'email' can \
                           return both values directly from the index — one I/O instead of two.\n\n\
                           Think of a covering index like a phone book that also lists email \
                           addresses. If you only need someone's phone number and email, the phone \
                           book covers your query completely. You never need to visit the person's \
                           house (the heap) to get that information. But the phone book is thicker \
                           (more storage) because it carries extra data."
                    .into(),
                algorithm: "BUILD INDEX:\n  \
                           1. For each incoming record:\n    \
                              a. Extract the key column value\n    \
                              b. Extract values for each included_column\n    \
                              c. Create a CoveringEntry with (key, covered_values map)\n    \
                              d. Insert into BTreeMap indexed by key string\n  \
                           2. Multiple records with the same key are stored in a Vec\n\n\
                           INDEX-ONLY LOOKUP:\n  \
                           1. Convert lookup_key to string and search in the BTreeMap\n  \
                           2. If found:\n    \
                              a. Increment index_only_scans counter\n    \
                              b. For each matching entry:\n      \
                                 - Build a Record with key + covered column values\n      \
                                 - Mark record with _index_only = true\n      \
                                 - Increment table_lookups_avoided\n    \
                              c. Return all matching records\n  \
                           3. If not found: return empty (the key does not exist)\n\n\
                           WHEN IS THE INDEX NOT COVERING?\n  \
                           If the query SELECTs a column that is NOT in included_columns,\n  \
                           the database must still fetch the full row from the heap table.\n  \
                           This is no different from a plain B-tree index lookup."
                    .into(),
                complexity: Complexity {
                    time: "Lookup O(log n), Range scan O(log n + k)".into(),
                    space: "O(n * (1 + included_columns)) — larger than a plain index".into(),
                },
                use_cases: vec![
                    "Queries that SELECT only indexed + included columns".into(),
                    "Avoiding expensive heap lookups on wide tables".into(),
                    "CREATE INDEX ... INCLUDE (col1, col2) in PostgreSQL".into(),
                    "High-frequency queries on hot paths where eliminating one I/O per query matters".into(),
                    "Reporting queries that always read the same small set of columns".into(),
                ],
                tradeoffs: vec![
                    "Eliminates table lookups for covered queries".into(),
                    "Larger index size due to stored column copies".into(),
                    "Updates to included columns must update the index too".into(),
                    "Diminishing returns when many columns are included — at some point the index is as big as the table".into(),
                    "Only beneficial when queries consistently use the same column subset; ad-hoc queries may not benefit".into(),
                ],
                examples: vec![
                    "PostgreSQL INCLUDE indexes — CREATE INDEX idx ON users (id) INCLUDE (name, email)".into(),
                    "SQL Server covering indexes — included columns stored only in leaf nodes".into(),
                    "MySQL composite indexes used as covering — leftmost prefix serves as key, remaining columns serve as included".into(),
                    "Oracle index-only access paths — optimizer detects when all needed columns are in the index".into(),
                ],
                motivation: "A standard B-tree index speeds up finding which rows match a predicate, \
                             but the database still has to visit the heap table to read the actual \
                             column values. For a query that matches 1000 rows, that means 1000 \
                             random I/O operations to the heap — each potentially a disk seek.\n\n\
                             The covering index solves this by storing copies of frequently-needed \
                             columns right in the index itself. The query can be answered entirely \
                             from the index with zero heap lookups. This trades extra storage space \
                             and write overhead for dramatically faster reads on specific query \
                             patterns. In practice, adding one or two INCLUDE columns to a hot \
                             index can reduce query latency by 50% or more."
                    .into(),
                parameter_guide: HashMap::from([
                    ("key_column".into(),
                     "The column to index on (the search key). This is the column used in WHERE \
                      clauses to find matching rows. The key determines the sort order of the \
                      index and enables both point lookups and range scans. Choose the column \
                      most frequently filtered on. Default is 'id'."
                         .into()),
                    ("included_columns".into(),
                     "Comma-separated list of columns to include in the index alongside the key. \
                      These columns are stored in the index leaf nodes so that queries requesting \
                      only these columns can be served without touching the base table (index-only \
                      scan). Include columns that are frequently in the SELECT list of queries \
                      that filter by the key_column. Adding too many columns makes the index \
                      bloated. A good rule of thumb: include 1-3 columns that cover your most \
                      common query pattern. Example: 'name,email' for a user lookup index."
                         .into()),
                    ("lookup_key".into(),
                     "The key value to search for during execution. When empty, the block only \
                      builds the index from incoming records without performing a lookup. When \
                      set, the block performs an index-only lookup after building. This simulates \
                      the two phases of index usage: build time (during INSERT) and query time \
                      (during SELECT). Default is empty (build only)."
                         .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "btree-index".into(),
                        comparison: "A plain B-tree index stores only keys and TupleIds. Lookups \
                                     require a follow-up heap access to get actual column values. \
                                     Choose a plain B-tree when your queries always need columns \
                                     not in the index (e.g., SELECT *). Choose a covering index \
                                     when your queries consistently SELECT a small set of columns \
                                     that can be included in the index."
                            .into(),
                    },
                    Alternative {
                        block_type: "hash-index".into(),
                        comparison: "Hash indexes provide O(1) equality lookups but store no extra \
                                     columns and cannot do range scans. Choose hash for the fastest \
                                     equality-only lookups when you will always fetch the full row. \
                                     Choose covering index when you want to avoid the heap lookup \
                                     entirely and need sorted access."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "How do I decide which columns to INCLUDE in a covering index?".into(),
                    "What is the storage overhead of a covering index compared to a plain B-tree index?".into(),
                    "When does a covering index stop being beneficial and become a liability?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "SQL Performance Explained by Markus Winand — Chapter 4".into(),
                url: None,
                citation: Some("Winand, M. (2012). SQL Performance Explained.".into()),
            }],
            icon: "book-open".into(),
            color: "#D97706".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(), name: "Records".into(), port_type: PortType::DataStream,
            direction: PortDirection::Input, required: true, multiple: false,
            description: "Records to index with included columns".into(), schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![
            Port {
                id: "index_results".into(), name: "Index Results".into(), port_type: PortType::DataStream,
                direction: PortDirection::Output, required: false, multiple: true,
                description: "Lookup results (index-only when possible)".into(), schema: None,
            },
        ]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "key_column".into(), name: "Key Column".into(), param_type: ParameterType::String,
                description: "Column to index on".into(),
                default_value: ParameterValue::String("id".into()),
                required: true, constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
            Parameter {
                id: "included_columns".into(), name: "Included Columns".into(),
                param_type: ParameterType::String,
                description: "Comma-separated columns to include in index (for index-only scans)".into(),
                default_value: ParameterValue::String("".into()),
                required: false, constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
            Parameter {
                id: "lookup_key".into(), name: "Lookup Key".into(), param_type: ParameterType::String,
                description: "Key value to look up (empty = build index only)".into(),
                default_value: ParameterValue::String("".into()),
                required: false, constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition { id: "total_entries".into(), name: "Total Entries".into(), metric_type: MetricType::Gauge, unit: "entries".into(), description: "Indexed entries".into(), aggregations: vec![AggregationType::Max] },
            MetricDefinition { id: "lookups".into(), name: "Lookups".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Index lookups".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "index_only_scans".into(), name: "Index-Only Scans".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Scans satisfied from index alone".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "table_lookups_avoided".into(), name: "Table Lookups Avoided".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Heap accesses saved".into(), aggregations: vec![AggregationType::Sum] },
        ]
    }

    fn total_entries(&self) -> usize {
        self.index.values().map(|v| v.len()).sum()
    }

    /// Build the index from records.
    fn build_index(&mut self, records: &[Record]) {
        for record in records {
            let key = record.data.get(&self.key_column).cloned().unwrap_or(JsonValue::Null);
            let key_str = key.to_string();

            let mut covered = HashMap::new();
            for col in &self.included_columns {
                if let Some(val) = record.data.get(col) {
                    covered.insert(col.clone(), val.clone());
                }
            }

            let entry = CoveringEntry { key, covered_values: covered };
            self.index.entry(key_str).or_default().push(entry);
        }
    }

    /// Lookup by key. Returns records built from index data only (index-only scan).
    fn lookup(&mut self, lookup_key: &str) -> Vec<Record> {
        self.lookups += 1;
        let mut results = Vec::new();

        if let Some(entries) = self.index.get(lookup_key) {
            self.index_only_scans += 1;
            self.table_lookups_avoided += entries.len();

            for entry in entries {
                let mut rec = Record::new();
                // Include the key column
                let _ = rec.data.insert(self.key_column.clone(), entry.key.clone());
                // Include covered columns
                for (k, v) in &entry.covered_values {
                    let _ = rec.data.insert(k.clone(), v.clone());
                }
                // Mark as index-only
                let _ = rec.data.insert("_index_only".into(), JsonValue::Bool(true));
                results.push(rec);
            }
        }
        results
    }
}

impl Default for CoveringIndexBlock { fn default() -> Self { Self::new() } }

#[async_trait]
impl Block for CoveringIndexBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] {
        static G: std::sync::LazyLock<Vec<Guarantee>> = std::sync::LazyLock::new(|| vec![
            Guarantee::strict(GuaranteeType::Consistency, "Index entries match source data at build time"),
        ]);
        &G
    }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(&mut self, params: HashMap<String, ParameterValue>) -> Result<(), BlockError> {
        if let Some(v) = params.get("key_column") {
            if let Some(s) = v.as_string() { self.key_column = s.to_string(); }
        }
        if let Some(v) = params.get("included_columns") {
            if let Some(s) = v.as_string() {
                self.included_columns = s.split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect();
            }
        }
        Ok(())
    }

    async fn execute(&mut self, context: ExecutionContext) -> Result<ExecutionResult, BlockError> {
        let records = match context.inputs.get("records").cloned().unwrap_or(PortValue::None) {
            PortValue::Stream(r) => r, PortValue::Batch(r) => r, PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        // Parse included_columns from context params if not set during initialize
        if self.included_columns.is_empty() {
            if let Some(v) = context.parameters.get("included_columns") {
                if let Some(s) = v.as_string() {
                    self.included_columns = s.split(',')
                        .map(|c| c.trim().to_string())
                        .filter(|c| !c.is_empty())
                        .collect();
                }
            }
        }

        // Build index from incoming records
        self.build_index(&records);

        // Check if a lookup is requested
        let lookup_key = context.parameters.get("lookup_key")
            .and_then(|v| v.as_string())
            .unwrap_or("");

        let output_records = if !lookup_key.is_empty() {
            self.lookup(lookup_key)
        } else {
            // No lookup — just pass through the original records
            records
        };

        context.metrics.record("total_entries", self.total_entries() as f64);
        context.metrics.record("lookups", self.lookups as f64);
        context.metrics.record("index_only_scans", self.index_only_scans as f64);
        context.metrics.record("table_lookups_avoided", self.table_lookups_avoided as f64);

        let mut outputs = HashMap::new();
        outputs.insert("index_results".into(), PortValue::Stream(output_records));
        let mut ms = HashMap::new();
        ms.insert("total_entries".into(), self.total_entries() as f64);
        ms.insert("lookups".into(), self.lookups as f64);
        ms.insert("index_only_scans".into(), self.index_only_scans as f64);

        Ok(ExecutionResult { outputs, metrics: ms, errors: vec![] })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if inputs.get("records").is_none() { ValidationResult::ok().with_warning("records input not connected") }
        else { ValidationResult::ok() }
    }
    fn get_state(&self) -> BlockState {
        let mut s = BlockState::new();
        let _ = s.insert("entries".into(), self.total_entries());
        s
    }
    fn set_state(&mut self, _: BlockState) -> Result<(), BlockError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_records() -> Vec<Record> {
        vec![
            { let mut r = Record::new(); r.insert("id".into(), 1_i64).unwrap(); r.insert("name".into(), "Alice").unwrap(); r.insert("email".into(), "alice@example.com").unwrap(); r },
            { let mut r = Record::new(); r.insert("id".into(), 2_i64).unwrap(); r.insert("name".into(), "Bob").unwrap(); r.insert("email".into(), "bob@example.com").unwrap(); r },
            { let mut r = Record::new(); r.insert("id".into(), 3_i64).unwrap(); r.insert("name".into(), "Charlie").unwrap(); r.insert("email".into(), "charlie@example.com").unwrap(); r },
            { let mut r = Record::new(); r.insert("id".into(), 1_i64).unwrap(); r.insert("name".into(), "Alice2").unwrap(); r.insert("email".into(), "alice2@example.com").unwrap(); r },
        ]
    }

    #[tokio::test]
    async fn test_build_and_lookup() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut ci = CoveringIndexBlock::new();
        ci.key_column = "id".into();
        ci.included_columns = vec!["name".into(), "email".into()];

        // Build
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(make_records()));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        ci.execute(ctx).await.unwrap();
        assert_eq!(ci.total_entries(), 4);

        // Lookup key "1" — should find 2 entries (Alice and Alice2)
        let mut params = HashMap::new();
        params.insert("lookup_key".into(), ParameterValue::String("1".into()));
        let ctx2 = ExecutionContext { inputs: HashMap::new(), parameters: params, metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = ci.execute(ctx2).await.unwrap();
        let results = match result.outputs.get("index_results").unwrap() { PortValue::Stream(r) => r.clone(), _ => panic!() };
        assert_eq!(results.len(), 2);
        // Should include covered columns
        assert!(results[0].data.contains_key("name"));
        assert!(results[0].data.contains_key("email"));
        assert_eq!(results[0].data.get("_index_only"), Some(&JsonValue::Bool(true)));
    }

    #[tokio::test]
    async fn test_index_only_metrics() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut ci = CoveringIndexBlock::new();
        ci.key_column = "id".into();
        ci.included_columns = vec!["name".into()];
        ci.build_index(&make_records());

        let results = ci.lookup("2");
        assert_eq!(results.len(), 1);
        assert_eq!(ci.lookups, 1);
        assert_eq!(ci.index_only_scans, 1);
        assert_eq!(ci.table_lookups_avoided, 1);
    }

    #[tokio::test]
    async fn test_lookup_miss() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut ci = CoveringIndexBlock::new();
        ci.key_column = "id".into();
        ci.included_columns = vec!["name".into()];
        ci.build_index(&make_records());

        let results = ci.lookup("999");
        assert_eq!(results.len(), 0);
        assert_eq!(ci.lookups, 1);
        assert_eq!(ci.index_only_scans, 0); // miss = no index-only scan
    }

    #[test]
    fn test_metadata() {
        let ci = CoveringIndexBlock::new();
        assert_eq!(ci.metadata().id, "covering-index");
        assert_eq!(ci.metadata().category, BlockCategory::Index);
    }
}
