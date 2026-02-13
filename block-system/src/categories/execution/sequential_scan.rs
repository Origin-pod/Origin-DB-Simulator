//! Sequential Scan Execution Block
//!
//! Reads all records from a storage block by iterating through every page.
//! This is the simplest scan strategy — no index required, but it must read
//! every page in the table.
//!
//! ## How it works
//!
//! The scan iterates through all input records, optionally applying a simple
//! predicate filter. Every record is examined regardless of whether it matches,
//! simulating the I/O cost of a full table scan.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `rows_scanned` | Counter | Total rows examined |
//! | `rows_returned` | Counter | Rows that passed the filter |
//! | `pages_read` | Counter | Simulated page reads |
//! | `selectivity` | Gauge | rows_returned / rows_scanned |

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
    Parameter, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// SequentialScanBlock
// ---------------------------------------------------------------------------

pub struct SequentialScanBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    filter_column: Option<String>,
    filter_value: Option<JsonValue>,
    records_per_page: usize,
}

impl SequentialScanBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            filter_column: None,
            filter_value: None,
            records_per_page: 100,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "sequential-scan".into(),
            name: "Sequential Scan".into(),
            category: BlockCategory::Execution,
            description: "Full table scan that reads every page sequentially".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A sequential scan (also called a full table scan or seq scan) reads \
                           every page of a table from first to last. It is the simplest and \
                           most fundamental scan strategy in any database — no index is needed, \
                           no special data structure is required. The scan simply walks through \
                           every page and examines every record.\n\n\
                           Sequential scans are the fallback plan when no suitable index exists \
                           for the query's predicates. But they are not always bad: for queries \
                           that need to touch most rows anyway (analytics, aggregations, reports), \
                           a sequential scan is actually the optimal choice because it reads \
                           pages in order, which is friendly to disk prefetching and OS read-ahead \
                           caches.\n\n\
                           Think of it like reading a phone book from cover to cover to find \
                           everyone named 'Smith'. You will definitely find all of them, but \
                           you have to read every single page. If 80% of entries are named \
                           Smith, this is perfectly reasonable. If only 1 entry is named Smith, \
                           you would rather use the index at the back of the book."
                    .into(),
                algorithm: "Sequential Scan Algorithm:\n\
                            \n\
                            FUNCTION sequential_scan(table, predicate):\n  \
                              results = []\n  \
                              pages_read = 0\n  \
                              FOR EACH page IN table.pages (in order):\n    \
                                pages_read += 1\n    \
                                FOR EACH record IN page.records:\n      \
                                  rows_scanned += 1\n      \
                                  IF predicate IS NULL OR predicate(record) == true:\n        \
                                    results.append(record)\n        \
                                    rows_returned += 1\n  \
                              selectivity = rows_returned / rows_scanned\n  \
                              RETURN results\n\
                            \n\
                            NOTE: Even when a predicate filters out most rows,\n\
                            ALL pages are still read — the I/O cost is the same."
                    .into(),
                complexity: Complexity {
                    time: "O(n) — must read every record".into(),
                    space: "O(1) — streams results, no buffering needed".into(),
                },
                use_cases: vec![
                    "Queries without an applicable index".into(),
                    "Aggregations that need all rows (COUNT(*), SUM)".into(),
                    "Small tables where index overhead isn't worthwhile".into(),
                    "OLAP/analytics queries that process the majority of the table".into(),
                    "Data export or ETL operations that read entire tables".into(),
                ],
                tradeoffs: vec![
                    "Simple and reliable but slow on large tables".into(),
                    "Predictable I/O pattern (sequential reads are cache-friendly)".into(),
                    "Doesn't benefit from selectivity — reads everything regardless".into(),
                    "Can pollute the buffer pool by loading pages that are only needed once, \
                     pushing out frequently accessed pages from other queries".into(),
                    "Performs well on SSDs since random vs sequential I/O gap is smaller, \
                     but still reads unnecessary data when selectivity is low".into(),
                ],
                examples: vec![
                    "PostgreSQL Seq Scan node — the query planner chooses this when the \
                     estimated selectivity is too high for an index scan to be worthwhile".into(),
                    "MySQL full table scan — chosen when no usable index covers the WHERE \
                     clause columns".into(),
                    "SQLite table scan — walks the B-tree leaf pages left to right".into(),
                ],
                motivation: "Every database needs a way to read data when no index is available. \
                             The sequential scan is the universal fallback that always works, \
                             regardless of what indexes exist. Without it, queries on un-indexed \
                             columns would simply fail.\n\n\
                             Sequential scans also serve as the baseline for understanding query \
                             performance: if an index scan is not significantly faster than a seq \
                             scan, the optimizer may choose the simpler scan instead. Understanding \
                             when and why a sequential scan is chosen is fundamental to query \
                             tuning."
                    .into(),
                parameter_guide: HashMap::from([
                    ("filter_column".into(), "The column name to filter on. When left empty, \
                                              no filter is applied and all rows are returned. \
                                              Setting a filter column simulates a WHERE clause — \
                                              the scan still reads every page, but only returns \
                                              rows where this column matches the filter value. \
                                              Use this to observe the difference between rows \
                                              scanned and rows returned.".into()),
                    ("filter_value".into(), "The value to match in the filter column. Only rows \
                                             where filter_column equals this value are returned. \
                                             Try different values to see how selectivity changes. \
                                             A highly selective value (matching few rows) shows \
                                             why an index scan would be better — the seq scan \
                                             still reads every page regardless.".into()),
                    ("records_per_page".into(), "Controls how many records fit on a simulated \
                                                 page. This affects the pages_read metric. Lower \
                                                 values simulate wider rows (fewer per page, more \
                                                 pages to read). Higher values simulate narrow rows. \
                                                 Typical databases fit 50-200 rows per 8 KB page \
                                                 depending on row width. Changing this helps you \
                                                 understand the relationship between row size and \
                                                 I/O cost.".into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "index-scan".into(),
                        comparison: "An index scan reads only the pages containing matching \
                                     records, making it far faster for selective queries (e.g., \
                                     WHERE id = 42). However, index scans use random I/O \
                                     (jumping between pages) which is slower per page than \
                                     sequential I/O. The crossover point is typically around \
                                     5-15% selectivity: below that, index scan wins; above it, \
                                     sequential scan is often faster. Choose sequential scan \
                                     for analytics and bulk reads; choose index scan for \
                                     selective point queries.".into(),
                    },
                ],
                suggested_questions: vec![
                    "At what selectivity percentage does a sequential scan become faster \
                     than an index scan, and why?".into(),
                    "How does the operating system's read-ahead optimization help sequential \
                     scans but not index scans?".into(),
                    "Why might a database choose a sequential scan even when an index exists \
                     on the filtered column?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 5: Query Processing".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "scan-line".into(),
            color: "#06B6D4".into(),
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
            description: "Records from a storage block to scan".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "results".into(),
            name: "Scan Results".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records that passed the scan filter".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "filter_column".into(),
                name: "Filter Column".into(),
                param_type: ParameterType::String,
                description: "Column to filter on (empty = no filter, return all)".into(),
                default_value: ParameterValue::String("".into()),
                required: false,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
            Parameter {
                id: "filter_value".into(),
                name: "Filter Value".into(),
                param_type: ParameterType::String,
                description: "Value to match in the filter column".into(),
                default_value: ParameterValue::String("".into()),
                required: false,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
            Parameter {
                id: "records_per_page".into(),
                name: "Records Per Page".into(),
                param_type: ParameterType::Number,
                description: "Simulated records per page (for page read counting)".into(),
                default_value: ParameterValue::Integer(100),
                required: false,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Slider)),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "rows_scanned".into(),
                name: "Rows Scanned".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Total rows examined".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "rows_returned".into(),
                name: "Rows Returned".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Rows that passed the filter".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "pages_read".into(),
                name: "Pages Read".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Simulated page reads".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "selectivity".into(),
                name: "Selectivity".into(),
                metric_type: MetricType::Gauge,
                unit: "%".into(),
                description: "Fraction of rows returned".into(),
                aggregations: vec![AggregationType::Avg],
            },
        ]
    }

    fn matches_filter(&self, record: &Record) -> bool {
        match (&self.filter_column, &self.filter_value) {
            (Some(col), Some(val)) => {
                record.data.get(col).map_or(false, |v| v == val)
            }
            _ => true, // No filter = return all
        }
    }
}

impl Default for SequentialScanBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Block for SequentialScanBlock {
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
        &[]
    }

    fn metrics(&self) -> &[MetricDefinition] {
        &self.metric_defs
    }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("filter_column") {
            if let Some(s) = val.as_string() {
                if !s.is_empty() {
                    self.filter_column = Some(s.to_string());
                }
            }
        }
        if let Some(val) = params.get("filter_value") {
            if let Some(s) = val.as_string() {
                if !s.is_empty() {
                    // Try to parse as number, else use as string.
                    self.filter_value = Some(
                        s.parse::<i64>()
                            .map(|n| JsonValue::Number(n.into()))
                            .unwrap_or_else(|_| JsonValue::String(s.to_string())),
                    );
                }
            }
        }
        if let Some(val) = params.get("records_per_page") {
            self.records_per_page = val
                .as_integer()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("records_per_page must be an integer".into())
                })? as usize;
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

        let rows_scanned = records.len();
        let pages_read = if self.records_per_page > 0 {
            (rows_scanned + self.records_per_page - 1) / self.records_per_page
        } else {
            rows_scanned
        };

        let mut results = Vec::new();
        for record in &records {
            if self.matches_filter(record) {
                results.push(record.clone());
            }
        }

        let rows_returned = results.len();
        let selectivity = if rows_scanned > 0 {
            (rows_returned as f64 / rows_scanned as f64) * 100.0
        } else {
            0.0
        };

        context
            .metrics
            .record("rows_scanned", rows_scanned as f64);
        context
            .metrics
            .record("rows_returned", rows_returned as f64);
        context.metrics.record("pages_read", pages_read as f64);
        context.metrics.record("selectivity", selectivity);

        let mut outputs = HashMap::new();
        outputs.insert("results".into(), PortValue::Stream(results));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("rows_scanned".into(), rows_scanned as f64);
        metrics_summary.insert("rows_returned".into(), rows_returned as f64);
        metrics_summary.insert("pages_read".into(), pages_read as f64);
        metrics_summary.insert("selectivity".into(), selectivity);

        Ok(ExecutionResult {
            outputs,
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
                    ValidationResult::ok().with_warning("No records to scan")
                }
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        BlockState::new()
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

    fn make_records(n: usize) -> Vec<Record> {
        (0..n)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
                r.insert("name".into(), format!("user_{}", i)).unwrap();
                r.insert("group".into(), if i % 2 == 0 { "even" } else { "odd" }).unwrap();
                r
            })
            .collect()
    }

    #[tokio::test]
    async fn test_scan_all() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut scan = SequentialScanBlock::new();
        scan.initialize(HashMap::new()).await.unwrap();

        let records = make_records(100);
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = scan.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_scanned").unwrap(), 100.0);
        assert_eq!(*result.metrics.get("rows_returned").unwrap(), 100.0);
        assert!(*result.metrics.get("selectivity").unwrap() > 99.0);
    }

    #[tokio::test]
    async fn test_scan_with_filter() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        use serde_json::json;

        let mut scan = SequentialScanBlock::new();
        scan.filter_column = Some("group".into());
        scan.filter_value = Some(json!("even"));

        let records = make_records(100);
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = scan.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_scanned").unwrap(), 100.0);
        assert_eq!(*result.metrics.get("rows_returned").unwrap(), 50.0);
        assert!((*result.metrics.get("selectivity").unwrap() - 50.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_page_counting() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut scan = SequentialScanBlock::new();
        scan.records_per_page = 10;

        let records = make_records(55);
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = scan.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("pages_read").unwrap(), 6.0); // ceil(55/10)
    }

    #[test]
    fn test_metadata() {
        let scan = SequentialScanBlock::new();
        assert_eq!(scan.metadata().id, "sequential-scan");
        assert_eq!(scan.metadata().category, BlockCategory::Execution);
    }
}
