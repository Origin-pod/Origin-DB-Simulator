//! Statistics Collector Block
//!
//! Gathers table and column statistics used by query planners to estimate
//! costs and choose optimal execution strategies. This is the foundation
//! for cost-based query optimization — PostgreSQL's ANALYZE command does this.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `rows_sampled` | Counter | Number of rows analyzed |
//! | `distinct_values` | Gauge | Estimated distinct values (cardinality) |
//! | `null_count` | Counter | NULL values encountered |
//! | `min_value` | Gauge | Minimum value seen |
//! | `max_value` | Gauge | Maximum value seen |
//! | `avg_row_width` | Gauge | Average row size in bytes |

use async_trait::async_trait;
use std::collections::HashMap;

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// StatisticsCollectorBlock
// ---------------------------------------------------------------------------

pub struct StatisticsCollectorBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    sample_rate: f64,
    histogram_buckets: usize,

    // Stats
    rows_sampled: usize,
    distinct_values: usize,
    null_count: usize,
    min_value: f64,
    max_value: f64,
}

impl StatisticsCollectorBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            sample_rate: 0.1,
            histogram_buckets: 100,
            rows_sampled: 0,
            distinct_values: 0,
            null_count: 0,
            min_value: f64::MAX,
            max_value: f64::MIN,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "statistics-collector".into(),
            name: "Statistics Collector".into(),
            category: BlockCategory::Optimization,
            description: "Gathers table/column statistics for cost-based query planning".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "The statistics collector analyzes table data to build statistics that query \
                           planners use for cost estimation. It samples rows, estimates cardinality \
                           (distinct values), tracks value distributions via histograms, and calculates \
                           selectivity estimates. This is what PostgreSQL's ANALYZE command and MySQL's \
                           ANALYZE TABLE do internally.\n\n\
                           In a database system, the statistics collector sits between the storage layer \
                           and the query optimizer. The optimizer needs to know things like: how many rows \
                           are in this table? How many distinct values does this column have? What fraction \
                           of rows match this WHERE clause? Without accurate statistics, the optimizer is \
                           flying blind and may choose catastrophically slow query plans.\n\n\
                           Think of it like a census for your database tables. Just as a government census \
                           samples the population to estimate demographics, the statistics collector samples \
                           rows to estimate data distribution. The results are stored in system catalogs \
                           (like pg_statistic in PostgreSQL) and consulted every time a query is planned."
                    .into(),
                algorithm: "COLLECT_STATISTICS(table, sample_rate):\n  \
                           1. Determine sample step: step = ceil(1.0 / sample_rate)\n  \
                           2. Initialize trackers:\n     \
                              distinct_set = HashSet (or HyperLogLog in production)\n     \
                              min_value = +infinity, max_value = -infinity\n     \
                              null_count = 0, total_width = 0\n  \
                           3. For each row at index i where i % step == 0:\n     \
                              a. Extract key value\n     \
                              b. If NULL: increment null_count\n     \
                              c. Else: add to distinct_set, update min/max\n     \
                              d. Accumulate row width estimate\n  \
                           4. Compute derived statistics:\n     \
                              cardinality = distinct_set.size()\n     \
                              null_fraction = null_count / rows_sampled\n     \
                              avg_width = total_width / rows_sampled\n  \
                           5. Build equi-depth histogram with configured bucket count\n  \
                           6. Store statistics in system catalog"
                    .into(),
                complexity: Complexity {
                    time: "O(n × sample_rate) — reads a fraction of the table".into(),
                    space: "O(distinct_values + histogram_buckets)".into(),
                },
                use_cases: vec![
                    "PostgreSQL's ANALYZE populates pg_statistic for the query planner".into(),
                    "MySQL's InnoDB samples pages for index statistics".into(),
                    "Cost-based optimizer uses cardinality to choose join order".into(),
                    "Detecting data skew — identifying columns with non-uniform distribution".into(),
                    "Triggering automatic re-optimization when statistics become stale".into(),
                ],
                tradeoffs: vec![
                    "Higher sample rate = more accurate but slower".into(),
                    "Stale statistics lead to suboptimal query plans".into(),
                    "More histogram buckets = better selectivity estimates".into(),
                    "Auto-analyze in PostgreSQL triggers after enough row changes".into(),
                    "Sampling introduces estimation error — rare values may be missed entirely".into(),
                    "Correlated columns are hard to capture with per-column statistics (multi-column stats help)".into(),
                ],
                examples: vec![
                    "PostgreSQL: ANALYZE tablename — stores results in pg_statistic, used by the planner for every query".into(),
                    "MySQL: ANALYZE TABLE tablename — updates index cardinality estimates in information_schema".into(),
                    "Oracle: DBMS_STATS.GATHER_TABLE_STATS — supports parallel sampling and histogram types".into(),
                    "SQL Server: UPDATE STATISTICS — auto-stats trigger after 20% of rows change".into(),
                ],
                motivation: "Without statistics, the query optimizer has no information about data distribution \
                             and must fall back to default assumptions. For example, it might assume every column \
                             has 1% selectivity for equality predicates — which could be wildly wrong. A query \
                             like 'WHERE country = US' on a table where 60% of rows are US would be estimated \
                             at 1% instead of 60%, leading the optimizer to choose a terrible plan.\n\n\
                             Stale or missing statistics are one of the most common causes of slow queries in \
                             production databases. Running ANALYZE regularly (or enabling auto-analyze) is one \
                             of the most impactful performance tuning actions a DBA can take."
                    .into(),
                parameter_guide: HashMap::from([
                    ("sample_rate".into(),
                     "The fraction of rows to sample during statistics collection. A value of 0.1 means \
                      10% of rows are sampled. Lower rates (0.01-0.05) are faster but may miss rare values \
                      and produce less accurate histograms. Higher rates (0.5-1.0) give more accurate \
                      statistics but take longer and use more memory. PostgreSQL's default_statistics_target \
                      controls a related concept (number of histogram buckets × 300 = rows sampled). \
                      Recommended: 0.1 for most tables, 0.5-1.0 for small tables or columns with high skew."
                        .into()),
                    ("histogram_buckets".into(),
                     "The number of equi-depth histogram buckets used to capture value distribution. More \
                      buckets provide finer-grained selectivity estimates but use more memory and take \
                      longer to compute. PostgreSQL defaults to 100 buckets (via default_statistics_target). \
                      For columns with uniform distribution, even 10 buckets suffice. For highly skewed \
                      columns, 200-500 buckets may be needed to capture the distribution accurately. \
                      Recommended: 100 for most columns, increase for skewed columns used in WHERE clauses."
                        .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "bloom-filter".into(),
                        comparison: "Statistics collectors and Bloom filters are both optimization tools but \
                                     serve entirely different purposes. Statistics collectors gather metadata \
                                     about data distribution (cardinality, histograms, min/max) to help the \
                                     query planner choose efficient join orders and access methods. Bloom \
                                     filters answer a specific question: 'is this key definitely NOT in this \
                                     data file?' Use statistics for query planning; use Bloom filters for \
                                     avoiding unnecessary disk reads during point lookups."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "How does PostgreSQL's autovacuum/autoanalyze decide when to refresh statistics?".into(),
                    "What is an equi-depth histogram, and how does it help estimate WHERE clause selectivity?".into(),
                    "What happens to query performance when statistics are stale or missing?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 13: Query Optimization".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "sparkles".into(),
            color: "#8B5CF6".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(),
            name: "Table Data".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Records to analyze for statistics collection".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "statistics".into(),
            name: "Statistics".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Statistics summary records with cardinality, distribution info".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "sample_rate".into(),
                name: "Sample Rate".into(),
                param_type: ParameterType::Number,
                description: "Fraction of rows to sample (0.01 = 1%, 1.0 = all rows)".into(),
                default_value: ParameterValue::Number(0.1),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(0.01).with_max(1.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(0.01)
                        .with_unit("ratio".into()),
                ),
            },
            Parameter {
                id: "histogram_buckets".into(),
                name: "Histogram Buckets".into(),
                param_type: ParameterType::Number,
                description: "Number of equi-depth histogram buckets".into(),
                default_value: ParameterValue::Integer(100),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(10.0).with_max(500.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider).with_step(10.0),
                ),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "rows_sampled".into(),
                name: "Rows Sampled".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Number of rows analyzed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "distinct_values".into(),
                name: "Distinct Values".into(),
                metric_type: MetricType::Gauge,
                unit: "values".into(),
                description: "Estimated number of distinct values (cardinality)".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "null_count".into(),
                name: "NULL Count".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Number of NULL values encountered".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "min_value".into(),
                name: "Min Value".into(),
                metric_type: MetricType::Gauge,
                unit: "".into(),
                description: "Minimum value in the sampled data".into(),
                aggregations: vec![AggregationType::Min],
            },
            MetricDefinition {
                id: "max_value".into(),
                name: "Max Value".into(),
                metric_type: MetricType::Gauge,
                unit: "".into(),
                description: "Maximum value in the sampled data".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "avg_row_width".into(),
                name: "Avg Row Width".into(),
                metric_type: MetricType::Gauge,
                unit: "bytes".into(),
                description: "Average row size in bytes (estimated)".into(),
                aggregations: vec![AggregationType::Avg],
            },
        ]
    }
}

impl Default for StatisticsCollectorBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for StatisticsCollectorBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] { &[] }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("sample_rate") {
            self.sample_rate = val
                .as_number()
                .ok_or_else(|| BlockError::InvalidParameter("sample_rate must be a number".into()))?;
        }
        if let Some(val) = params.get("histogram_buckets") {
            self.histogram_buckets = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("histogram_buckets must be an integer".into()))?
                as usize;
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context.inputs.get("records").cloned().unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        let total_rows = records.len();
        let mut distinct_set = std::collections::HashSet::new();
        let mut total_width: usize = 0;

        // Simple deterministic sampling: take every Nth row.
        let step = if self.sample_rate >= 1.0 {
            1
        } else {
            (1.0 / self.sample_rate).ceil() as usize
        };

        for (i, record) in records.iter().enumerate() {
            if i % step != 0 {
                continue;
            }
            self.rows_sampled += 1;

            // Get _key for distinct value tracking.
            if let Ok(Some(key)) = record.get::<u64>("_key") {
                distinct_set.insert(key);
                let fkey = key as f64;
                if fkey < self.min_value { self.min_value = fkey; }
                if fkey > self.max_value { self.max_value = fkey; }
            } else {
                self.null_count += 1;
            }

            // Estimate row width (number of fields × 8 bytes average).
            total_width += record.data.len() * 8;
        }

        self.distinct_values = distinct_set.len();
        let avg_width = if self.rows_sampled > 0 {
            total_width as f64 / self.rows_sampled as f64
        } else {
            0.0
        };

        // Build output statistics record.
        let mut stats = Record::new();
        let _ = stats.insert("_total_rows".into(), total_rows);
        let _ = stats.insert("_rows_sampled".into(), self.rows_sampled);
        let _ = stats.insert("_distinct_values".into(), self.distinct_values);
        let _ = stats.insert("_null_count".into(), self.null_count);
        let _ = stats.insert("_avg_row_width".into(), avg_width as usize);

        context.metrics.record("rows_sampled", self.rows_sampled as f64);
        context.metrics.record("distinct_values", self.distinct_values as f64);
        context.metrics.record("null_count", self.null_count as f64);
        if self.min_value != f64::MAX {
            context.metrics.record("min_value", self.min_value);
        }
        if self.max_value != f64::MIN {
            context.metrics.record("max_value", self.max_value);
        }
        context.metrics.record("avg_row_width", avg_width);

        let mut outputs = HashMap::new();
        outputs.insert("statistics".into(), PortValue::Single(stats));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("rows_sampled".into(), self.rows_sampled as f64);
        metrics_summary.insert("distinct_values".into(), self.distinct_values as f64);
        metrics_summary.insert("null_count".into(), self.null_count as f64);
        if self.min_value != f64::MAX {
            metrics_summary.insert("min_value".into(), self.min_value);
        }
        if self.max_value != f64::MIN {
            metrics_summary.insert("max_value".into(), self.max_value);
        }
        metrics_summary.insert("avg_row_width".into(), avg_width);

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("records") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => ValidationResult::ok(),
                PortValue::None => ValidationResult::ok().with_warning("No data to analyze"),
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("sample_rate".into(), self.sample_rate);
        let _ = state.insert("histogram_buckets".into(), self.histogram_buckets);
        let _ = state.insert("rows_sampled".into(), self.rows_sampled);
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(r)) = state.get::<f64>("sample_rate") { self.sample_rate = r; }
        if let Ok(Some(b)) = state.get::<usize>("histogram_buckets") { self.histogram_buckets = b; }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_collection() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut collector = StatisticsCollectorBlock::new();
        collector.sample_rate = 1.0; // Sample everything

        let records: Vec<Record> = (0..100u64).map(|i| {
            let mut r = Record::new();
            r.insert("_key".into(), i).unwrap();
            r
        }).collect();

        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = collector.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_sampled").unwrap(), 100.0);
        assert_eq!(*result.metrics.get("distinct_values").unwrap(), 100.0);
    }

    #[test]
    fn test_metadata() {
        let sc = StatisticsCollectorBlock::new();
        assert_eq!(sc.metadata().id, "statistics-collector");
        assert_eq!(sc.metadata().category, BlockCategory::Optimization);
    }
}
