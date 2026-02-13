//! Clustered Storage Block
//!
//! Stores records physically ordered by a **cluster key** (like InnoDB's
//! clustered index). Records with adjacent key values share the same page,
//! making range scans on the cluster key very fast.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `pages_used` | Gauge | Pages containing records |
//! | `records_stored` | Gauge | Total records stored |
//! | `inserts_in_order` | Counter | Inserts that fit page order |
//! | `page_splits` | Counter | Page splits from out-of-order inserts |

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
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

fn cmp_json(a: &JsonValue, b: &JsonValue) -> std::cmp::Ordering {
    match (a, b) {
        (JsonValue::Number(na), JsonValue::Number(nb)) => {
            let fa = na.as_f64().unwrap_or(0.0);
            let fb = nb.as_f64().unwrap_or(0.0);
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        }
        (JsonValue::String(sa), JsonValue::String(sb)) => sa.cmp(sb),
        _ => a.to_string().cmp(&b.to_string()),
    }
}

pub struct ClusteredStorageBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    cluster_key: String,
    page_size: usize,

    /// Sorted storage: cluster_key → record data. BTreeMap keeps keys sorted.
    store: BTreeMap<String, JsonValue>,
    page_splits: usize,
    inserts_in_order: usize,
    last_key_value: Option<JsonValue>,
}

impl ClusteredStorageBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            cluster_key: "id".into(),
            page_size: 100,
            store: BTreeMap::new(),
            page_splits: 0,
            inserts_in_order: 0,
            last_key_value: None,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "clustered-storage".into(),
            name: "Clustered Storage".into(),
            category: BlockCategory::Storage,
            description: "Records physically ordered by cluster key for fast range scans".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "Clustered storage keeps records physically sorted by a designated cluster \
                           key on disk. Unlike a heap file where records land on whichever page has \
                           space, clustered storage ensures that records with adjacent key values are \
                           stored on adjacent pages. This means range scans on the cluster key become \
                           fast sequential I/O instead of random page fetches.\n\n\
                           In relational databases, a clustered index defines the physical order of \
                           the table itself — the table IS the index. MySQL InnoDB, for example, \
                           always organizes every table as a clustered index on the primary key. \
                           Secondary indexes then point to the primary key rather than to a physical \
                           page location.\n\n\
                           Think of clustered storage like a dictionary: words are stored in \
                           alphabetical order, so finding all words starting with 'M' means \
                           flipping to the M section and reading sequentially. But inserting a \
                           new word in the middle means shifting everything after it — which is \
                           why out-of-order inserts cause expensive page splits."
                    .into(),
                algorithm: "INSERT:\n  \
                           1. Extract the cluster key value from the record\n  \
                           2. Compare with the last inserted key value\n  \
                           3. If new key >= last key: in-order insert (append to current position)\n  \
                           4. If new key < last key: out-of-order insert (page split simulated)\n  \
                           5. Insert into BTreeMap which maintains sorted order automatically\n  \
                           6. Update last_key_value for next comparison\n\n\
                           RANGE SCAN (on cluster key):\n  \
                           1. Seek to the start key in the sorted structure (O(log n))\n  \
                           2. Read sequentially until end key is reached\n  \
                           3. All matching records are physically contiguous — minimal I/O\n\n\
                           PAGE SPLIT (out-of-order insert):\n  \
                           1. When a record arrives with a key smaller than existing entries\n  \
                           2. The page containing that key range must be split\n  \
                           3. Half the entries move to a new page to make room\n  \
                           4. This is expensive — it requires rewriting two pages"
                    .into(),
                complexity: Complexity {
                    time: "Range scan O(k) for k matching pages, Insert O(log n)".into(),
                    space: "O(n) — one copy of each record".into(),
                },
                use_cases: vec![
                    "Primary key range queries".into(),
                    "Time-series data ordered by timestamp".into(),
                    "Sequential access patterns".into(),
                    "Tables where most queries filter or sort by the primary key".into(),
                    "Foreign key lookups in parent-child relationships (e.g., all orders for a customer)".into(),
                ],
                tradeoffs: vec![
                    "Fast range scans on cluster key, slow on other columns".into(),
                    "Random inserts cause page splits".into(),
                    "Only one cluster key per table".into(),
                    "Secondary index lookups require a double lookup (index -> PK -> row)".into(),
                    "Choosing a poor cluster key (e.g., UUID) leads to constant page splits and fragmentation".into(),
                ],
                examples: vec![
                    "MySQL InnoDB — every table is clustered by the primary key; secondary indexes store the PK value".into(),
                    "SQL Server clustered indexes — explicitly created, default on PRIMARY KEY constraint".into(),
                    "Oracle Index-Organized Tables (IOT) — opt-in clustered storage for specific tables".into(),
                ],
                motivation: "Without clustered storage, range queries on a key must perform random I/O \
                             — fetching pages scattered across disk for each matching row. For a query \
                             like SELECT * FROM orders WHERE customer_id BETWEEN 100 AND 200, a heap \
                             file would require jumping to a different page for each matching order.\n\n\
                             Clustered storage solves this by physically co-locating records with similar \
                             key values. The same range query now reads a contiguous run of pages \
                             sequentially, which can be 10-100x faster on spinning disks and still \
                             significantly faster on SSDs due to prefetching and reduced seeks."
                    .into(),
                parameter_guide: HashMap::from([
                    ("cluster_key".into(),
                     "The column used to physically sort all records. This is the most important \
                      decision for a clustered table. Choose a column that is frequently used in \
                      range queries, ORDER BY clauses, or JOIN conditions. Monotonically increasing \
                      keys (like auto-increment integers or timestamps) are ideal because inserts \
                      always append to the end, avoiding page splits. UUIDs are a poor choice \
                      because they are random, causing constant page splits and fragmentation. \
                      Default is 'id'."
                         .into()),
                    ("page_size".into(),
                     "Number of records per logical page. Larger pages (e.g., 1000-10000) reduce \
                      the number of pages and metadata overhead but mean more data is moved during \
                      a page split. Smaller pages (e.g., 10-50) result in more frequent page \
                      splits but each split is cheaper. Recommended: 100-1000 for most workloads. \
                      Range: 10-10000. Default is 100."
                         .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "heap-file-storage".into(),
                        comparison: "Heap files store records in insertion order with no sorting. \
                                     They have faster inserts (no page splits) but range scans \
                                     require full table scans or an index. Choose heap when insert \
                                     throughput matters more than range scan speed, or when you will \
                                     rely on separate indexes for all queries."
                            .into(),
                    },
                    Alternative {
                        block_type: "lsm-tree-storage".into(),
                        comparison: "LSM trees buffer writes in memory and flush sorted runs, \
                                     achieving very high write throughput. They support range scans \
                                     but data is spread across levels, making scans more expensive \
                                     than clustered storage. Choose LSM for write-dominated workloads. \
                                     Choose clustered when range scan performance is the top priority."
                            .into(),
                    },
                    Alternative {
                        block_type: "columnar-storage".into(),
                        comparison: "Columnar storage organizes data by column rather than by row. \
                                     It is designed for analytical queries that aggregate over few \
                                     columns. Choose columnar for OLAP. Choose clustered for OLTP \
                                     workloads with frequent range queries on the primary key."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "Why does using UUIDs as the cluster key cause performance problems?".into(),
                    "How does a page split work, and why is it expensive?".into(),
                    "What is the difference between a clustered index and a secondary index in InnoDB?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 3".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "table".into(),
            color: "#059669".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port { id: "records".into(), name: "Records".into(), port_type: PortType::DataStream, direction: PortDirection::Input, required: true, multiple: false, description: "Records to store in clustered order".into(), schema: None }]
    }
    fn build_outputs() -> Vec<Port> {
        vec![Port { id: "stored".into(), name: "Stored Records".into(), port_type: PortType::DataStream, direction: PortDirection::Output, required: false, multiple: true, description: "Records in clustered order".into(), schema: None }]
    }
    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter { id: "cluster_key".into(), name: "Cluster Key".into(), param_type: ParameterType::String, description: "Column to cluster by".into(), default_value: ParameterValue::String("id".into()), required: true, constraints: None, ui_hint: Some(ParameterUIHint::new(WidgetType::Input)) },
            Parameter { id: "page_size".into(), name: "Page Size".into(), param_type: ParameterType::Number, description: "Records per page".into(), default_value: ParameterValue::Integer(100), required: false, constraints: Some(ParameterConstraints::new().with_min(10.0).with_max(10000.0)), ui_hint: Some(ParameterUIHint::new(WidgetType::Slider).with_step(10.0)) },
        ]
    }
    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition { id: "pages_used".into(), name: "Pages Used".into(), metric_type: MetricType::Gauge, unit: "pages".into(), description: "Pages with records".into(), aggregations: vec![AggregationType::Max] },
            MetricDefinition { id: "records_stored".into(), name: "Records Stored".into(), metric_type: MetricType::Gauge, unit: "records".into(), description: "Total records".into(), aggregations: vec![AggregationType::Max] },
            MetricDefinition { id: "inserts_in_order".into(), name: "In-Order Inserts".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Inserts that maintained order".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "page_splits".into(), name: "Page Splits".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Out-of-order page splits".into(), aggregations: vec![AggregationType::Sum] },
        ]
    }

    fn pages_used(&self) -> usize {
        if self.store.is_empty() { 0 }
        else { (self.store.len() + self.page_size - 1) / self.page_size }
    }
}

impl Default for ClusteredStorageBlock { fn default() -> Self { Self::new() } }

#[async_trait]
impl Block for ClusteredStorageBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] {
        static G: std::sync::LazyLock<Vec<Guarantee>> = std::sync::LazyLock::new(|| vec![Guarantee::strict(GuaranteeType::Consistency, "Records stored in cluster key order")]);
        &G
    }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(&mut self, params: HashMap<String, ParameterValue>) -> Result<(), BlockError> {
        if let Some(v) = params.get("cluster_key") { if let Some(s) = v.as_string() { self.cluster_key = s.to_string(); } }
        if let Some(v) = params.get("page_size") { self.page_size = v.as_integer().unwrap_or(100) as usize; }
        Ok(())
    }

    async fn execute(&mut self, context: ExecutionContext) -> Result<ExecutionResult, BlockError> {
        let records = match context.inputs.get("records").cloned().unwrap_or(PortValue::None) {
            PortValue::Stream(r) => r, PortValue::Batch(r) => r, PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        for record in &records {
            let key_value = record.data.get(&self.cluster_key).cloned().unwrap_or(JsonValue::Null);
            let key_str = key_value.to_string();
            let in_order = self.last_key_value.as_ref().map_or(true, |lk| {
                cmp_json(&key_value, lk) != std::cmp::Ordering::Less
            });
            if in_order { self.inserts_in_order += 1; } else { self.page_splits += 1; }
            self.last_key_value = Some(key_value);
            self.store.insert(key_str, serde_json::to_value(&record.data).unwrap_or(JsonValue::Null));
        }

        context.metrics.record("pages_used", self.pages_used() as f64);
        context.metrics.record("records_stored", self.store.len() as f64);
        context.metrics.record("inserts_in_order", self.inserts_in_order as f64);
        context.metrics.record("page_splits", self.page_splits as f64);

        let mut outputs = HashMap::new();
        outputs.insert("stored".into(), PortValue::Stream(records));
        let mut ms = HashMap::new();
        ms.insert("records_stored".into(), self.store.len() as f64);
        ms.insert("pages_used".into(), self.pages_used() as f64);
        ms.insert("page_splits".into(), self.page_splits as f64);

        Ok(ExecutionResult { outputs, metrics: ms, errors: vec![] })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if inputs.get("records").is_none() { ValidationResult::ok().with_warning("records input not connected") }
        else { ValidationResult::ok() }
    }
    fn get_state(&self) -> BlockState { let mut s = BlockState::new(); let _ = s.insert("records".into(), self.store.len()); s }
    fn set_state(&mut self, _: BlockState) -> Result<(), BlockError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clustered_insert_and_order() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut cs = ClusteredStorageBlock::new();
        cs.cluster_key = "id".into();
        let records: Vec<Record> = (0..50).map(|i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r }).collect();
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = cs.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("records_stored").unwrap(), 50.0);
        assert_eq!(cs.page_splits, 0, "Sequential inserts should not cause splits");
    }

    #[tokio::test]
    async fn test_out_of_order_inserts() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut cs = ClusteredStorageBlock::new();
        cs.cluster_key = "id".into();
        let records: Vec<Record> = [5, 3, 8, 1].iter().map(|&i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r }).collect();
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        cs.execute(ctx).await.unwrap();
        assert!(cs.page_splits > 0, "Out-of-order inserts should cause splits");
    }

    #[test]
    fn test_metadata() {
        let cs = ClusteredStorageBlock::new();
        assert_eq!(cs.metadata().id, "clustered-storage");
        assert_eq!(cs.metadata().category, BlockCategory::Storage);
    }
}
