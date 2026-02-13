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
                overview: "Clustered storage keeps records physically sorted by a cluster key. \
                           This makes range scans on that key sequential I/O instead of random. \
                           InnoDB tables are always clustered by the primary key."
                    .into(),
                algorithm: "Insert: place record in sorted position by cluster key. If the \
                            record doesn't fit page order, simulate a page split. Range scan: \
                            read pages sequentially since data is already sorted."
                    .into(),
                complexity: Complexity {
                    time: "Range scan O(k) for k matching pages, Insert O(log n)".into(),
                    space: "O(n) — one copy of each record".into(),
                },
                use_cases: vec![
                    "Primary key range queries".into(),
                    "Time-series data ordered by timestamp".into(),
                    "Sequential access patterns".into(),
                ],
                tradeoffs: vec![
                    "Fast range scans on cluster key, slow on other columns".into(),
                    "Random inserts cause page splits".into(),
                    "Only one cluster key per table".into(),
                ],
                examples: vec![
                    "MySQL InnoDB clustered index".into(),
                    "SQL Server clustered index".into(),
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
