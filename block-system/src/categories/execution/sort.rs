//! Sort Execution Block
//!
//! Sorts input records by a specified column. Simulates both in-memory and
//! external sort (when data exceeds a configurable memory limit).
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `rows_sorted` | Counter | Total rows sorted |
//! | `comparisons` | Counter | Key comparisons made |
//! | `external_runs` | Gauge | Merge runs for external sort |
//! | `sort_type` | Gauge | 0 = in-memory, 1 = external |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::core::block::{
    Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
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

pub struct SortBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    sort_column: String,
    descending: bool,
    memory_limit: usize, // Max records for in-memory sort
}

impl SortBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            sort_column: "id".into(),
            descending: false,
            memory_limit: 10000,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "sort".into(),
            name: "Sort".into(),
            category: BlockCategory::Execution,
            description: "Sort records by a column with in-memory or external sort".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A sort operator orders input records by a specified column. When \
                           the data fits in memory, a standard O(n log n) sort is used. For \
                           larger datasets, an external merge sort divides data into sorted \
                           runs that are merged."
                    .into(),
                algorithm: "In-memory: standard sort (Rust's pdqsort). External: split input \
                            into sorted runs of memory_limit size, then k-way merge the runs."
                    .into(),
                complexity: Complexity {
                    time: "O(n log n) in-memory, O(n log n × log(n/M)) external".into(),
                    space: "O(n) in-memory, O(M) external where M = memory limit".into(),
                },
                use_cases: vec![
                    "ORDER BY clause".into(),
                    "Input to merge join".into(),
                    "Top-K queries (with limit)".into(),
                ],
                tradeoffs: vec![
                    "Blocking operator — must read all input before producing output".into(),
                    "External sort is slower but handles arbitrarily large data".into(),
                ],
                examples: vec!["PostgreSQL Sort node".into(), "MySQL filesort".into()],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 6: Sorting".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "arrow-up-down".into(),
            color: "#6366F1".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(), name: "Records".into(), port_type: PortType::DataStream,
            direction: PortDirection::Input, required: true, multiple: false,
            description: "Records to sort".into(), schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "sorted".into(), name: "Sorted Records".into(), port_type: PortType::DataStream,
            direction: PortDirection::Output, required: false, multiple: true,
            description: "Records in sorted order".into(), schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "sort_column".into(), name: "Sort Column".into(), param_type: ParameterType::String,
                description: "Column to sort by".into(), default_value: ParameterValue::String("id".into()),
                required: true, constraints: None, ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
            Parameter {
                id: "descending".into(), name: "Descending".into(), param_type: ParameterType::Boolean,
                description: "Sort in descending order".into(), default_value: ParameterValue::Boolean(false),
                required: false, constraints: None, ui_hint: Some(ParameterUIHint::new(WidgetType::Checkbox)),
            },
            Parameter {
                id: "memory_limit".into(), name: "Memory Limit".into(), param_type: ParameterType::Number,
                description: "Max records for in-memory sort (exceeding triggers external sort)".into(),
                default_value: ParameterValue::Integer(10000), required: false,
                constraints: Some(ParameterConstraints::new().with_min(10.0).with_max(1000000.0)),
                ui_hint: Some(ParameterUIHint::new(WidgetType::Slider).with_step(100.0).with_unit("records".into())),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition { id: "rows_sorted".into(), name: "Rows Sorted".into(), metric_type: MetricType::Counter, unit: "rows".into(), description: "Total rows sorted".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "comparisons".into(), name: "Comparisons".into(), metric_type: MetricType::Counter, unit: "ops".into(), description: "Key comparisons made".into(), aggregations: vec![AggregationType::Sum] },
            MetricDefinition { id: "external_runs".into(), name: "External Runs".into(), metric_type: MetricType::Gauge, unit: "runs".into(), description: "Merge runs for external sort".into(), aggregations: vec![AggregationType::Max] },
            MetricDefinition { id: "sort_type".into(), name: "Sort Type".into(), metric_type: MetricType::Gauge, unit: "".into(), description: "0 = in-memory, 1 = external".into(), aggregations: vec![AggregationType::Max] },
        ]
    }
}

impl Default for SortBlock {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Block for SortBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] { &[] }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(&mut self, params: HashMap<String, ParameterValue>) -> Result<(), BlockError> {
        if let Some(v) = params.get("sort_column") { if let Some(s) = v.as_string() { self.sort_column = s.to_string(); } }
        if let Some(v) = params.get("descending") { if let Some(b) = v.as_bool() { self.descending = b; } }
        if let Some(v) = params.get("memory_limit") { self.memory_limit = v.as_integer().unwrap_or(10000) as usize; }
        Ok(())
    }

    async fn execute(&mut self, context: ExecutionContext) -> Result<ExecutionResult, BlockError> {
        let mut records = match context.inputs.get("records").cloned().unwrap_or(PortValue::None) {
            PortValue::Stream(r) => r, PortValue::Batch(r) => r, PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        let rows = records.len();
        let is_external = rows > self.memory_limit;
        let external_runs = if is_external { (rows + self.memory_limit - 1) / self.memory_limit } else { 0 };

        let mut comparisons = 0usize;
        let col = self.sort_column.clone();
        let desc = self.descending;

        if is_external {
            // Simulate external sort: sort each run, then merge.
            let mut runs: Vec<Vec<Record>> = records.chunks(self.memory_limit).map(|c| {
                let mut chunk = c.to_vec();
                chunk.sort_by(|a, b| {
                    comparisons += 1;
                    let va = a.data.get(&col).unwrap_or(&JsonValue::Null);
                    let vb = b.data.get(&col).unwrap_or(&JsonValue::Null);
                    let ord = cmp_json(va, vb);
                    if desc { ord.reverse() } else { ord }
                });
                chunk
            }).collect();

            // K-way merge (simplified: merge all runs into one sorted vec).
            records = Vec::with_capacity(rows);
            loop {
                let mut best_run = None;
                let mut best_val = None;
                for (i, run) in runs.iter().enumerate() {
                    if let Some(rec) = run.first() {
                        let v = rec.data.get(&col).unwrap_or(&JsonValue::Null).clone();
                        comparisons += 1;
                        if best_val.is_none() || {
                            let ord = cmp_json(&v, best_val.as_ref().unwrap());
                            if desc { ord == std::cmp::Ordering::Greater } else { ord == std::cmp::Ordering::Less }
                        } {
                            best_run = Some(i);
                            best_val = Some(v);
                        }
                    }
                }
                match best_run {
                    Some(i) => records.push(runs[i].remove(0)),
                    None => break,
                }
            }
        } else {
            records.sort_by(|a, b| {
                comparisons += 1;
                let va = a.data.get(&col).unwrap_or(&JsonValue::Null);
                let vb = b.data.get(&col).unwrap_or(&JsonValue::Null);
                let ord = cmp_json(va, vb);
                if desc { ord.reverse() } else { ord }
            });
        }

        context.metrics.record("rows_sorted", rows as f64);
        context.metrics.record("comparisons", comparisons as f64);
        context.metrics.record("external_runs", external_runs as f64);
        context.metrics.record("sort_type", if is_external { 1.0 } else { 0.0 });

        let mut outputs = HashMap::new();
        outputs.insert("sorted".into(), PortValue::Stream(records));
        let mut ms = HashMap::new();
        ms.insert("rows_sorted".into(), rows as f64);
        ms.insert("comparisons".into(), comparisons as f64);
        ms.insert("external_runs".into(), external_runs as f64);

        Ok(ExecutionResult { outputs, metrics: ms, errors: vec![] })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if inputs.get("records").is_none() { ValidationResult::ok().with_warning("records input not connected") }
        else { ValidationResult::ok() }
    }
    fn get_state(&self) -> BlockState { BlockState::new() }
    fn set_state(&mut self, _: BlockState) -> Result<(), BlockError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_records() -> Vec<Record> {
        [5, 3, 8, 1, 9, 2, 7, 4, 6, 0].iter().map(|&i| {
            let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r
        }).collect()
    }

    #[tokio::test]
    async fn test_sort_ascending() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut s = SortBlock::new();
        s.sort_column = "id".into();
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(make_records()));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = s.execute(ctx).await.unwrap();
        let sorted = match result.outputs.get("sorted").unwrap() { PortValue::Stream(r) => r.clone(), _ => panic!() };
        for i in 0..sorted.len()-1 {
            let a = sorted[i].get::<i64>("id").unwrap().unwrap();
            let b = sorted[i+1].get::<i64>("id").unwrap().unwrap();
            assert!(a <= b, "Not sorted: {} > {}", a, b);
        }
    }

    #[tokio::test]
    async fn test_sort_descending() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut s = SortBlock::new();
        s.sort_column = "id".into(); s.descending = true;
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(make_records()));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = s.execute(ctx).await.unwrap();
        let sorted = match result.outputs.get("sorted").unwrap() { PortValue::Stream(r) => r.clone(), _ => panic!() };
        for i in 0..sorted.len()-1 {
            let a = sorted[i].get::<i64>("id").unwrap().unwrap();
            let b = sorted[i+1].get::<i64>("id").unwrap().unwrap();
            assert!(a >= b, "Not sorted desc: {} < {}", a, b);
        }
    }

    #[tokio::test]
    async fn test_external_sort() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut s = SortBlock::new();
        s.sort_column = "id".into(); s.memory_limit = 3;
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(make_records()));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = s.execute(ctx).await.unwrap();
        assert!(*result.metrics.get("external_runs").unwrap() > 0.0);
        let sorted = match result.outputs.get("sorted").unwrap() { PortValue::Stream(r) => r.clone(), _ => panic!() };
        for i in 0..sorted.len()-1 {
            let a = sorted[i].get::<i64>("id").unwrap().unwrap();
            let b = sorted[i+1].get::<i64>("id").unwrap().unwrap();
            assert!(a <= b);
        }
    }

    #[test]
    fn test_metadata() {
        let s = SortBlock::new();
        assert_eq!(s.metadata().id, "sort");
        assert_eq!(s.metadata().category, BlockCategory::Execution);
    }
}
