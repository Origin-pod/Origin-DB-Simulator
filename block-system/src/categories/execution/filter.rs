//! Filter Execution Block
//!
//! Evaluates a predicate on each input record and passes through only matching
//! rows. Supports comparison operators on a single column.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `rows_in` | Counter | Total rows evaluated |
//! | `rows_out` | Counter | Rows that passed the filter |
//! | `selectivity` | Gauge | rows_out / rows_in as percentage |

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
    Parameter, ParameterType, ParameterUIHint, ParameterValue, ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// FilterBlock
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub struct FilterBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    column: String,
    op: FilterOp,
    value: JsonValue,
}

impl FilterBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            column: "id".into(),
            op: FilterOp::Eq,
            value: JsonValue::Null,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "filter".into(),
            name: "Filter".into(),
            category: BlockCategory::Execution,
            description: "Predicate filter that passes only matching rows".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A filter operator evaluates a predicate on each input row and \
                           outputs only the rows that match. This corresponds to the WHERE \
                           clause in SQL."
                    .into(),
                algorithm: "For each input record, evaluate column <op> value. If true, \
                            emit the record to output. O(n) with no extra space."
                    .into(),
                complexity: Complexity {
                    time: "O(n) — evaluates predicate on each row".into(),
                    space: "O(1) — streaming, no buffering".into(),
                },
                use_cases: vec![
                    "WHERE clause evaluation".into(),
                    "Post-scan filtering".into(),
                    "HAVING clause in aggregations".into(),
                ],
                tradeoffs: vec![
                    "Simple and fast but must evaluate every input row".into(),
                    "Push-down to storage can avoid reading unneeded pages".into(),
                ],
                examples: vec!["PostgreSQL Filter node".into(), "MySQL WHERE evaluation".into()],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database System Concepts — Chapter 13: Query Processing".into(),
                url: None,
                citation: Some("Silberschatz, A. et al. (2019). McGraw-Hill.".into()),
            }],
            icon: "filter".into(),
            color: "#EC4899".into(),
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
            description: "Records to filter".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "results".into(),
            name: "Filtered Results".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records that passed the filter".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "column".into(),
                name: "Column".into(),
                param_type: ParameterType::String,
                description: "Column to filter on".into(),
                default_value: ParameterValue::String("id".into()),
                required: true,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
            Parameter {
                id: "operator".into(),
                name: "Operator".into(),
                param_type: ParameterType::String,
                description: "Comparison operator (eq, ne, lt, le, gt, ge)".into(),
                default_value: ParameterValue::String("eq".into()),
                required: true,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Select)),
            },
            Parameter {
                id: "value".into(),
                name: "Value".into(),
                param_type: ParameterType::String,
                description: "Value to compare against".into(),
                default_value: ParameterValue::String("".into()),
                required: true,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "rows_in".into(),
                name: "Rows In".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Total rows evaluated".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "rows_out".into(),
                name: "Rows Out".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Rows that passed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "selectivity".into(),
                name: "Selectivity".into(),
                metric_type: MetricType::Gauge,
                unit: "%".into(),
                description: "Fraction of rows that passed".into(),
                aggregations: vec![AggregationType::Avg],
            },
        ]
    }

    fn evaluate(&self, record: &Record) -> bool {
        let field = match record.data.get(&self.column) {
            Some(v) => v,
            None => return false,
        };
        match &self.op {
            FilterOp::Eq => field == &self.value,
            FilterOp::Ne => field != &self.value,
            FilterOp::Lt => cmp_json(field, &self.value) == std::cmp::Ordering::Less,
            FilterOp::Le => cmp_json(field, &self.value) != std::cmp::Ordering::Greater,
            FilterOp::Gt => cmp_json(field, &self.value) == std::cmp::Ordering::Greater,
            FilterOp::Ge => cmp_json(field, &self.value) != std::cmp::Ordering::Less,
        }
    }
}

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

fn parse_op(s: &str) -> FilterOp {
    match s.to_lowercase().as_str() {
        "ne" | "!=" | "<>" => FilterOp::Ne,
        "lt" | "<" => FilterOp::Lt,
        "le" | "<=" => FilterOp::Le,
        "gt" | ">" => FilterOp::Gt,
        "ge" | ">=" => FilterOp::Ge,
        _ => FilterOp::Eq,
    }
}

impl Default for FilterBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Block for FilterBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] { &[] }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(&mut self, params: HashMap<String, ParameterValue>) -> Result<(), BlockError> {
        if let Some(v) = params.get("column") { if let Some(s) = v.as_string() { self.column = s.to_string(); } }
        if let Some(v) = params.get("operator") { if let Some(s) = v.as_string() { self.op = parse_op(s); } }
        if let Some(v) = params.get("value") {
            if let Some(s) = v.as_string() {
                self.value = s.parse::<i64>()
                    .map(|n| JsonValue::Number(n.into()))
                    .or_else(|_| s.parse::<f64>().map(|f| serde_json::Number::from_f64(f).map(JsonValue::Number).unwrap_or(JsonValue::String(s.to_string()))))
                    .unwrap_or_else(|_| JsonValue::String(s.to_string()));
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

        let rows_in = records.len();
        let results: Vec<Record> = records.into_iter().filter(|r| self.evaluate(r)).collect();
        let rows_out = results.len();
        let selectivity = if rows_in > 0 { (rows_out as f64 / rows_in as f64) * 100.0 } else { 0.0 };

        context.metrics.record("rows_in", rows_in as f64);
        context.metrics.record("rows_out", rows_out as f64);
        context.metrics.record("selectivity", selectivity);

        let mut outputs = HashMap::new();
        outputs.insert("results".into(), PortValue::Stream(results));
        let mut ms = HashMap::new();
        ms.insert("rows_in".into(), rows_in as f64);
        ms.insert("rows_out".into(), rows_out as f64);
        ms.insert("selectivity".into(), selectivity);

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
        (0..10).map(|i| { let mut r = Record::new(); r.insert("id".into(), i as i64).unwrap(); r.insert("name".into(), format!("u{}", i)).unwrap(); r }).collect()
    }

    #[tokio::test]
    async fn test_filter_eq() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut f = FilterBlock::new();
        f.column = "id".into(); f.op = FilterOp::Eq; f.value = json!(5);
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(make_records()));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = f.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_out").unwrap(), 1.0);
    }

    #[tokio::test]
    async fn test_filter_lt() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
        let mut f = FilterBlock::new();
        f.column = "id".into(); f.op = FilterOp::Lt; f.value = json!(5);
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(make_records()));
        let ctx = ExecutionContext { inputs, parameters: HashMap::new(), metrics: MetricsCollector::new(), logger: Logger::new(), storage: StorageContext::new() };
        let result = f.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_out").unwrap(), 5.0); // 0,1,2,3,4
    }

    #[test]
    fn test_metadata() {
        let f = FilterBlock::new();
        assert_eq!(f.metadata().id, "filter");
        assert_eq!(f.metadata().category, BlockCategory::Execution);
    }
}
