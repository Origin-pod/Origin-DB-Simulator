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
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
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
                overview: "Columnar storage organizes data by column rather than by row. Instead \
                           of storing all fields of a record together (as in a heap file), each \
                           column is stored as a separate contiguous array. This means analytical \
                           queries that only need a few columns can skip reading the rest entirely, \
                           dramatically reducing I/O.\n\n\
                           In a database system, columnar storage is the foundation of OLAP (Online \
                           Analytical Processing) engines and data warehouses. When you run a query \
                           like SELECT AVG(price) FROM sales WHERE year = 2024, a row-oriented \
                           engine must read every column of every row. A columnar engine reads only \
                           the 'price' and 'year' columns, skipping all others. This also enables \
                           excellent compression because values within a column are of the same type \
                           and often have low cardinality (e.g., a 'country' column with only 200 \
                           distinct values across millions of rows).\n\n\
                           Think of columnar storage like a spreadsheet where each column is stored \
                           in its own file. If you need to sum column C across a million rows, you \
                           only open file C — not files A, B, D, E. The tradeoff is that fetching \
                           a complete row requires opening every column file and reconstructing it."
                    .into(),
                algorithm: "INGEST (decompose rows into columns):\n  \
                           1. For each incoming record:\n    \
                              a. For each (key, value) in the record:\n      \
                                 - Find or create the Column for that key\n      \
                                 - If column is new, backfill NULLs for previously ingested rows\n      \
                                 - Append the value to the column's value array\n    \
                              b. For columns not present in this record, push NULL\n    \
                              c. Increment row_count\n\n\
                           PROJECT (reconstruct rows from selected columns):\n  \
                           1. Determine which columns to read (empty projection = all)\n  \
                           2. For row index i = 0..row_count:\n    \
                              a. Create a new Record\n    \
                              b. For each selected column, read values[i]\n    \
                              c. Emit the reconstructed record\n  \
                           3. Track columns_read for I/O metrics\n\n\
                           COMPRESSION ESTIMATION:\n  \
                           1. For each column, count distinct values (cardinality)\n  \
                           2. compression_ratio = total_values / distinct_values\n  \
                           3. Low cardinality columns (e.g., status, category) compress very well"
                    .into(),
                complexity: Complexity {
                    time: "Insert O(c) per row where c = columns, Projection O(n) for n rows".into(),
                    space: "O(n * c) — same total data, different layout".into(),
                },
                use_cases: vec![
                    "OLAP / analytical queries (aggregations over few columns)".into(),
                    "Data warehousing (star schema fact tables)".into(),
                    "Column-level compression for low-cardinality data".into(),
                    "Time-series analytics where queries aggregate over specific metrics".into(),
                    "Business intelligence dashboards that compute rollups and summaries".into(),
                ],
                tradeoffs: vec![
                    "Fast column scans but slow single-row lookups".into(),
                    "Excellent compression for low-cardinality columns".into(),
                    "Row reconstruction requires reading multiple columns".into(),
                    "Updates are expensive — append-mostly workloads preferred".into(),
                    "Schema changes (adding columns) require backfilling NULLs for existing rows".into(),
                    "Not suitable for OLTP workloads that read/write individual rows frequently".into(),
                ],
                examples: vec![
                    "Apache Parquet — the standard columnar file format for data lakes and Spark/Hadoop".into(),
                    "ClickHouse MergeTree — high-performance columnar OLAP database with real-time ingestion".into(),
                    "Amazon Redshift — cloud data warehouse using columnar storage with zone maps".into(),
                    "DuckDB — in-process columnar analytics database, often called 'SQLite for analytics'".into(),
                ],
                motivation: "Row-oriented storage engines (heap files, B-trees) must read entire rows \
                             even when a query only needs one or two columns. For a table with 100 \
                             columns, a query that aggregates a single column wastes 99% of the I/O \
                             bandwidth reading irrelevant data.\n\n\
                             Columnar storage solves this by physically separating columns so that \
                             only the requested columns are read from disk. For analytical workloads \
                             that scan millions of rows but touch only a few columns, this can reduce \
                             I/O by 10-100x. Additionally, since all values in a column share the \
                             same data type, columnar storage achieves much better compression ratios \
                             than row-oriented storage."
                    .into(),
                parameter_guide: HashMap::from([
                    ("projection".into(),
                     "A comma-separated list of column names to include in the output. When empty, \
                      all columns are projected (like SELECT *). When specified, only those columns \
                      are read, simulating the I/O savings of columnar storage. For example, \
                      'id,price' reads only those two columns. This is the key advantage of \
                      columnar layout: you pay I/O cost only for the columns you actually need. \
                      Try different projections to see how columns_read changes in the metrics."
                         .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "heap-file-storage".into(),
                        comparison: "Heap files store complete rows together, making single-row \
                                     reads fast but column-only scans wasteful. Choose heap for \
                                     OLTP workloads that read/write full rows. Choose columnar for \
                                     analytical queries that scan many rows but only need a few \
                                     columns."
                            .into(),
                    },
                    Alternative {
                        block_type: "clustered-storage".into(),
                        comparison: "Clustered storage sorts rows by a key for fast range scans \
                                     but still reads entire rows. Choose clustered for OLTP range \
                                     queries on the primary key. Choose columnar for OLAP aggregation \
                                     queries that only need specific columns."
                            .into(),
                    },
                    Alternative {
                        block_type: "lsm-tree-storage".into(),
                        comparison: "LSM trees optimize for write throughput with key-value semantics. \
                                     They are row-oriented and not designed for column projections. \
                                     Choose LSM for write-heavy key-value workloads. Choose columnar \
                                     for read-heavy analytical workloads with selective column access."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "Why does columnar storage compress better than row-oriented storage?".into(),
                    "How much I/O is saved by projecting 2 columns out of 20 in a columnar layout vs. a row layout?".into(),
                    "What are zone maps and how do they help columnar engines skip irrelevant data blocks?".into(),
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
