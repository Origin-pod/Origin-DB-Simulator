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
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
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
                overview: "A filter operator evaluates a boolean predicate on each input row \
                           and passes through only the rows that satisfy the condition. In SQL \
                           terms, this is the WHERE clause — the gatekeeper that decides which \
                           rows participate in the rest of the query.\n\n\
                           Filters are streaming operators: they process one row at a time, \
                           immediately emitting or discarding each row without needing to buffer \
                           the entire input. This makes them memory-efficient and composable — \
                           you can chain multiple filters together, and each one reduces the \
                           data volume for downstream operators.\n\n\
                           In a query plan, the filter typically appears after a scan (sequential \
                           or index) and before operators like sort, join, or aggregation. The \
                           query optimizer tries to push filters as early as possible in the plan \
                           (called 'predicate pushdown') to reduce the amount of data flowing \
                           through expensive operators."
                    .into(),
                algorithm: "Filter Algorithm:\n\
                            \n\
                            FUNCTION filter(records, column, op, value):\n  \
                              results = []\n  \
                              FOR EACH record IN records:\n    \
                                field = record[column]\n    \
                                IF field IS NULL:\n      \
                                  SKIP  // NULL never matches\n    \
                                match = EVALUATE(field, op, value)\n    \
                                  // op is one of: eq, ne, lt, le, gt, ge\n    \
                                IF match:\n      \
                                  results.append(record)\n  \
                              selectivity = results.len / records.len * 100\n  \
                              RETURN results\n\
                            \n\
                            NOTE: This is a streaming operator — records can be\n\
                            emitted immediately without waiting for all input."
                    .into(),
                complexity: Complexity {
                    time: "O(n) — evaluates predicate on each row".into(),
                    space: "O(1) — streaming, no buffering".into(),
                },
                use_cases: vec![
                    "WHERE clause evaluation".into(),
                    "Post-scan filtering".into(),
                    "HAVING clause in aggregations".into(),
                    "Join predicate residual filtering (non-equi conditions after a join)".into(),
                    "Security row-level filtering (row-level security policies)".into(),
                ],
                tradeoffs: vec![
                    "Simple and fast but must evaluate every input row".into(),
                    "Push-down to storage can avoid reading unneeded pages".into(),
                    "Filter ordering matters: putting the most selective filter first reduces \
                     the number of rows evaluated by subsequent filters".into(),
                    "Complex predicates (LIKE, regex, function calls) are more expensive per \
                     row than simple comparisons — consider index support for these".into(),
                ],
                examples: vec![
                    "PostgreSQL Filter node — shown in EXPLAIN output when a predicate cannot \
                     be pushed into the scan itself".into(),
                    "MySQL WHERE evaluation — applied after the storage engine returns rows".into(),
                    "SQLite WHERE clause — evaluated during the virtual machine bytecode execution".into(),
                ],
                motivation: "Without a filter operator, queries would return every row from the \
                             scanned table, and the application would have to discard unwanted \
                             rows itself. This wastes network bandwidth, memory, and CPU time. \
                             Filters push the selection logic into the database engine, reducing \
                             the data volume as early as possible in the query pipeline.\n\n\
                             Filters are also the key to query optimization: the selectivity of \
                             a filter (what fraction of rows pass) determines which join strategy, \
                             scan type, and access path the optimizer chooses. Understanding \
                             selectivity is fundamental to understanding query plans."
                    .into(),
                parameter_guide: HashMap::from([
                    ("column".into(), "The name of the column to evaluate the predicate against. \
                                       This must match a field name in the input records. If the \
                                       column does not exist in a record, that record is filtered \
                                       out (treated as non-matching). Try different columns to see \
                                       how selectivity changes based on data distribution.".into()),
                    ("operator".into(), "The comparison operator to use. Supported values: \
                                         'eq' (=), 'ne' (!=), 'lt' (<), 'le' (<=), 'gt' (>), \
                                         'ge' (>=). Equality checks (eq) are the most common in \
                                         OLTP queries, while range operators (lt, gt, etc.) are \
                                         common in analytics. The operator affects selectivity: \
                                         equality on a unique column gives exactly 1 row, while \
                                         a range might match thousands.".into()),
                    ("value".into(), "The value to compare against. This is parsed as an integer \
                                      if possible, otherwise as a float, otherwise as a string. \
                                      The type must be compatible with the column's data type for \
                                      meaningful comparisons. For example, comparing a string \
                                      column with a numeric value may produce unexpected results.".into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "index-scan".into(),
                        comparison: "An index scan combines the filter predicate with data access: \
                                     instead of reading all rows and filtering, it uses an index to \
                                     find only matching rows. This is much faster for selective predicates \
                                     but requires an index on the filter column. A standalone filter \
                                     is used when no suitable index exists, or when additional predicates \
                                     remain after an index scan has been applied.".into(),
                    },
                    Alternative {
                        block_type: "sequential-scan".into(),
                        comparison: "A sequential scan with a built-in filter predicate achieves the \
                                     same result as a scan followed by a separate filter block. The \
                                     separate filter block is useful in the visual pipeline to make \
                                     the filtering step explicit and tunable independently of the scan.".into(),
                    },
                ],
                suggested_questions: vec![
                    "What is predicate pushdown, and why does pushing a filter closer to the \
                     storage layer improve performance?".into(),
                    "How does the selectivity of a filter affect the query optimizer's choice \
                     between a sequential scan and an index scan?".into(),
                    "Why does filter ordering matter when multiple WHERE conditions are applied, \
                     and how do databases decide which filter to evaluate first?".into(),
                ],
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
