//! Columnar Storage Block
//!
//! Stores data in **column-oriented** format instead of row-oriented. Each
//! column is stored as a separate contiguous array, which is ideal for
//! analytical queries that read only a few columns from many rows.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `columns_stored` | Gauge | Number of distinct columns |
//! | `rows_stored` | Gauge | Total rows stored |
//! | `columns_read` | Counter | Column reads (projections) |
//! | `compression_ratio` | Gauge | Simulated compression ratio |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::core::block::{
    Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee, GuaranteeType};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterType, ParameterUIHint, ParameterValue, ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

/// A single column stored as a contiguous vector of JSON values.
#[derive(Debug, Clone)]
struct Column {
    name: String,
    values: Vec<JsonValue>,
}

impl Column {
    fn new(name: String) -> Self {
        Self { name, values: Vec::new() }
    }

    /// Estimate compression ratio based on value repetition.
    /// Columnar stores compress well when there's low cardinality.
    fn compression_ratio(&self) -> f64 {
        if self.values.is_empty() { return 1.0; }
        let mut unique = self.values.clone();
        unique.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        unique.dedup_by(|a, b| a.to_string() == b.to_string());
        let cardinality = unique.len() as f64;
        let total = self.values.len() as f64;
        // Higher ratio = better compression (more duplicates)
        if cardinality == 0.0 { 1.0 } else { total / cardinality }
    }
}

pub struct ColumnarStorageBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    /// Column-oriented storage: column_name → Column
    columns: HashMap<String, Column>,
    row_count: usize,
    columns_read: usize,
}

impl ColumnarStorageBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            columns: HashMap::new(),
            row_count: 0,
            columns_read: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "columnar-storage".into(),
            name: "Columnar Storage".into(),
            category: BlockCategory::Storage,
            description: "Column-oriented storage for analytical workloads".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "Columnar storage organizes data by column rather than by row. Each \
                           column is stored as a contiguous array, which means analytical queries \
                           that only need a few columns can skip reading the rest. This also \
                           enables excellent compression since similar values are stored together."
                    .into(),
                algorithm: "Insert: decompose each row into individual column values and append. \
                            Read: project only the requested columns, reconstructing rows. \
                            Compression is simulated by measuring value cardinality per column."
                    .into(),
                complexity: Complexity {
                    time: "Insert O(c) per row where c = columns, Projection O(n) for n rows".into(),
                    space: "O(n × c) — same total data, different layout".into(),
                },
                use_cases: vec![
                    "OLAP / analytical queries (aggregations over few columns)".into(),
                    "Data warehousing (star schema fact tables)".into(),
                    "Column-level compression for low-cardinality data".into(),
                ],
                tradeoffs: vec![
                    "Fast column scans but slow single-row lookups".into(),
                    "Excellent compression for low-cardinality columns".into(),
                    "Row reconstruction requires reading multiple columns".into(),
                    "Updates are expensive — append-mostly workloads preferred".into(),
                ],
                examples: vec![
                    "Apache Parquet file format".into(),
                    "ClickHouse MergeTree engine".into(),
                    "Amazon Redshift columnar storage".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Designing Data-Intensive Applications — Chapter 3: Column-Oriented Storage".into(),
                url: None,
                citation: Some("Kleppmann, M. (2017). DDIA. O'Reilly.".into()),
            }],
            icon: "columns".into(),
            color: "#8B5CF6".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(), name: "Records".into(), port_type: PortType::DataStream,
            direction: PortDirection::Input, required: true, multiple: false,
            description: "Records to decompose into columnar format".into(), schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "projected".into(), name: "Projected Records".into(), port_type: PortType::DataStream,
            direction: PortDirection::Output, required: false, multiple: true,
            description: "Records reconstructed from selected columns".into(), schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "projection".into(), name: "Projection Columns".into(),
            param_type: ParameterType::String,
            description: "Comma-separated column names to project (empty = all)".into(),
            default_value: ParameterValue::String("".into()),
            required: false, constraints: None,
            ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition { id: "columns_stored".into(), name: "Columns Stored".into(), metric_type: MetricType::Gauge, unit: "columns".into(), description: "Distinct columns".into(), aggregations: vec![AggregationType::Max] },
            MetricDefinition { id: "rows_stored".into(), name: "Rows Stored".into(), metric_type: MetricType::Gauge, unit: "rows".into(), description: "Total rows".into(), aggregations: vec![AggregationType::Max] },
            MetricDefinition { id: "columns_read".into(), name: "Columns Read".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Column projections performed".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "compression_ratio".into(), name: "Compression Ratio".into(), metric_type: MetricType::Gauge, unit: "x".into(), description: "Average compression ratio across columns".into(), aggregations: vec![AggregationType::Max] },
        ]
    }

    fn avg_compression_ratio(&self) -> f64 {
        if self.columns.is_empty() { return 1.0; }
        let sum: f64 = self.columns.values().map(|c| c.compression_ratio()).sum();
        sum / self.columns.len() as f64
    }

    /// Store rows by decomposing into columns.
    fn ingest(&mut self, records: &[Record]) {
        for record in records {
            for (key, value) in &record.data {
                let col = self.columns
                    .entry(key.clone())
                    .or_insert_with(|| {
                        let mut c = Column::new(key.clone());
                        // Backfill NULLs for rows already ingested
                        for _ in 0..self.row_count {
                            c.values.push(JsonValue::Null);
                        }
                        c
                    });
                col.values.push(value.clone());
            }
            // For columns not present in this record, push NULL
            for col in self.columns.values_mut() {
                if col.values.len() <= self.row_count {
                    col.values.push(JsonValue::Null);
                }
            }
            self.row_count += 1;
        }
    }

    /// Project selected columns back into rows.
    fn project(&mut self, col_names: &[String]) -> Vec<Record> {
        let cols: Vec<&String> = if col_names.is_empty() {
            self.columns.keys().collect()
        } else {
            col_names.iter().filter(|n| self.columns.contains_key(*n)).collect()
        };

        self.columns_read += cols.len();

        let mut result = Vec::with_capacity(self.row_count);
        for i in 0..self.row_count {
            let mut rec = Record::new();
            for col_name in &cols {
                if let Some(col) = self.columns.get(*col_name) {
                    let val = col.values.get(i).cloned().unwrap_or(JsonValue::Null);
                    let _ = rec.data.insert((*col_name).clone(), val);
                }
            }
            result.push(rec);
        }
        result
    }
}

impl Default for ColumnarStorageBlock { fn default() -> Self { Self::new() } }

#[async_trait]
impl Block for ColumnarStorageBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] {
        static G: std::sync::LazyLock<Vec<Guarantee>> = std::sync::LazyLock::new(|| vec![
            Guarantee::best_effort(GuaranteeType::Consistency, "Columnar layout preserves all values"),
        ]);
        &G
    }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(&mut self, _params: HashMap<String, ParameterValue>) -> Result<(), BlockError> {
        Ok(())
    }

    async fn execute(&mut self, context: ExecutionContext) -> Result<ExecutionResult, BlockError> {
        let records = match context.inputs.get("records").cloned().unwrap_or(PortValue::None) {
            PortValue::Stream(r) => r, PortValue::Batch(r) => r, PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        // Ingest into columnar format
        self.ingest(&records);

        // Parse projection parameter
        let projection_str = context.parameters.get("projection")
            .and_then(|v| v.as_string())
            .unwrap_or("");
        let projection_cols: Vec<String> = if projection_str.is_empty() {
            Vec::new() // empty = all columns
        } else {
            projection_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        };

        let projected = self.project(&projection_cols);

        context.metrics.record("columns_stored", self.columns.len() as f64);
        context.metrics.record("rows_stored", self.row_count as f64);
        context.metrics.record("columns_read", self.columns_read as f64);
        context.metrics.record("compression_ratio", self.avg_compression_ratio());

        let mut outputs = HashMap::new();
        outputs.insert("projected".into(), PortValue::Stream(projected));
        let mut ms = HashMap::new();
        ms.insert("rows_stored".into(), self.row_count as f64);
        ms.insert("columns_stored".into(), self.columns.len() as f64);
        ms.insert("compression_ratio".into(), self.avg_compression_ratio());

        Ok(ExecutionResult { outputs, metrics: ms, errors: vec![] })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if inputs.get("records").is_none() { ValidationResult::ok().with_warning("records input not connected") }
        else { ValidationResult::ok() }
    }
    fn get_state(&self) -> BlockState {
        let mut s = BlockState::new();
        let _ = s.insert("rows".into(), self.row_count);
        let _ = s.insert("columns".into(), self.columns.len());
        s
    }
    fn set_state(&mut self, _: BlockState) -> Result<(), BlockError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_records() -> Vec<Record> {
        (0..10).map(|i| {
            let mut r = Record::new();
            r.insert("id".into(), i as i64).unwrap();
            r.insert("name".into(), format!("user_{}", i)).unwrap();
            r.insert("category".into(), if i % 3 == 0 { "A" } else { "B" }).unwrap();
            r
        }).collect()
    }

    #[tokio::test]
    async fn test_columnar_ingest_and_project() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut col = ColumnarStorageBlock::new();
        let records = make_records();
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = col.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_stored").unwrap(), 10.0);
        assert_eq!(*result.metrics.get("columns_stored").unwrap(), 3.0);
        let projected = match result.outputs.get("projected").unwrap() { PortValue::Stream(r) => r.clone(), _ => panic!() };
        assert_eq!(projected.len(), 10);
        // All columns projected (empty projection = all)
        assert_eq!(projected[0].data.len(), 3);
    }

    #[tokio::test]
    async fn test_columnar_selective_projection() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut col = ColumnarStorageBlock::new();
        col.ingest(&make_records());

        let mut params = HashMap::new();
        params.insert("projection".into(), ParameterValue::String("id,name".into()));
        let inputs = HashMap::new();
        let ctx = ExecutionContext { inputs, parameters: params, metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = col.execute(ctx).await.unwrap();
        let projected = match result.outputs.get("projected").unwrap() { PortValue::Stream(r) => r.clone(), _ => panic!() };
        // Only id and name columns projected
        assert_eq!(projected[0].data.len(), 2);
        assert!(projected[0].data.contains_key("id"));
        assert!(projected[0].data.contains_key("name"));
        assert!(!projected[0].data.contains_key("category"));
    }

    #[tokio::test]
    async fn test_compression_ratio() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut col = ColumnarStorageBlock::new();
        // category column has low cardinality (only "A" and "B") → should compress well
        let records = make_records();
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = col.execute(ctx).await.unwrap();
        let ratio = *result.metrics.get("compression_ratio").unwrap();
        assert!(ratio > 1.0, "Should have compression ratio > 1 due to repeated values");
    }

    #[test]
    fn test_metadata() {
        let col = ColumnarStorageBlock::new();
        assert_eq!(col.metadata().id, "columnar-storage");
        assert_eq!(col.metadata().category, BlockCategory::Storage);
    }
}
