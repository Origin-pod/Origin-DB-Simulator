//! Replication Block
//!
//! Simulates writing data to multiple replicas with configurable consistency
//! levels. Fundamental to every distributed database — Cassandra, MongoDB,
//! DynamoDB, and PostgreSQL streaming replication all use variants of this.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `writes_replicated` | Counter | Writes sent to replicas |
//! | `acks_received` | Counter | Acknowledgments received |
//! | `replication_lag_ms` | Gauge | Simulated lag for async replication |
//! | `consistency_met` | Counter | Writes that met the consistency level |
//! | `consistency_violations` | Counter | Writes that didn't meet consistency |

use async_trait::async_trait;
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
use crate::core::port::{Port, PortDirection, PortType, PortValue};

// ---------------------------------------------------------------------------
// ReplicationBlock
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConsistencyLevel {
    One,
    Quorum,
    All,
}

pub struct ReplicationBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    replication_factor: usize,
    consistency_level: ConsistencyLevel,
    async_replication: bool,

    // Stats
    writes_replicated: usize,
    acks_received: usize,
    consistency_met: usize,
    consistency_violations: usize,
}

impl ReplicationBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            replication_factor: 3,
            consistency_level: ConsistencyLevel::Quorum,
            async_replication: false,
            writes_replicated: 0,
            acks_received: 0,
            consistency_met: 0,
            consistency_violations: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "replication".into(),
            name: "Replication".into(),
            category: BlockCategory::Distribution,
            description: "Writes data to multiple replicas with configurable consistency".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "Replication copies data to multiple nodes for fault tolerance and \
                           availability. The consistency level determines how many replicas \
                           must acknowledge a write before it's considered successful. This \
                           is the core of the CAP theorem — you trade consistency for \
                           availability and partition tolerance."
                    .into(),
                algorithm: "On write: send to all R replicas. Wait for acks based on consistency \
                            level. ONE = 1 ack, QUORUM = ⌊R/2⌋+1 acks, ALL = R acks. \
                            Async mode sends to one replica and propagates to others in background."
                    .into(),
                complexity: Complexity {
                    time: "O(R) per write where R is replication factor; quorum latency = max of fastest ⌊R/2⌋+1 replicas".into(),
                    space: "O(R × data_size) — R copies of every record".into(),
                },
                use_cases: vec![
                    "Cassandra writes to R replicas with tunable consistency".into(),
                    "MongoDB replica sets: primary + secondaries".into(),
                    "DynamoDB replicates across 3 AZs automatically".into(),
                    "PostgreSQL streaming replication for hot standby".into(),
                ],
                tradeoffs: vec![
                    "Higher replication factor = better durability but more storage and write latency".into(),
                    "ALL consistency = strong consistency but one slow replica blocks writes".into(),
                    "ONE consistency = low latency but stale reads possible".into(),
                    "QUORUM read + QUORUM write = linearizable consistency (R+W > N)".into(),
                    "Async replication = lower latency but data loss risk on failure".into(),
                ],
                examples: vec![
                    "Cassandra: replication_factor=3, consistency=QUORUM".into(),
                    "MongoDB: 3-member replica set with write concern 'majority'".into(),
                    "DynamoDB: always replicates to 3 AZs".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Designing Data-Intensive Applications by Martin Kleppmann — Chapter 5: Replication".into(),
                url: None,
                citation: Some("Kleppmann, M. (2017). Designing Data-Intensive Applications. O'Reilly.".into()),
            }],
            icon: "network".into(),
            color: "#06B6D4".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "requests".into(),
            name: "Write Requests".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Records to replicate across nodes".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "replicated".into(),
            name: "Replicated Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records enriched with `_replicas` and `_acks` fields".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "replication_factor".into(),
                name: "Replication Factor".into(),
                param_type: ParameterType::Number,
                description: "Number of copies to maintain (typical: 3)".into(),
                default_value: ParameterValue::Integer(3),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(1.0).with_max(7.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider).with_step(1.0),
                ),
            },
            Parameter {
                id: "consistency_level".into(),
                name: "Consistency Level".into(),
                param_type: ParameterType::String,
                description: "How many replicas must ack: one, quorum, or all".into(),
                default_value: ParameterValue::String("quorum".into()),
                required: false,
                constraints: None,
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Select),
                ),
            },
            Parameter {
                id: "async_replication".into(),
                name: "Async Mode".into(),
                param_type: ParameterType::Boolean,
                description: "If true, ack after writing to one replica (lower latency, risk of data loss)".into(),
                default_value: ParameterValue::Boolean(false),
                required: false,
                constraints: None,
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Checkbox),
                ),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "writes_replicated".into(),
                name: "Writes Replicated".into(),
                metric_type: MetricType::Counter,
                unit: "writes".into(),
                description: "Total writes sent to replica set".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "acks_received".into(),
                name: "Acks Received".into(),
                metric_type: MetricType::Counter,
                unit: "acks".into(),
                description: "Total acknowledgments from replicas".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "replication_lag_ms".into(),
                name: "Replication Lag".into(),
                metric_type: MetricType::Gauge,
                unit: "ms".into(),
                description: "Simulated replication lag for async mode".into(),
                aggregations: vec![AggregationType::Avg],
            },
            MetricDefinition {
                id: "consistency_met".into(),
                name: "Consistency Met".into(),
                metric_type: MetricType::Counter,
                unit: "writes".into(),
                description: "Writes that met the required consistency level".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "consistency_violations".into(),
                name: "Consistency Violations".into(),
                metric_type: MetricType::Counter,
                unit: "writes".into(),
                description: "Writes that didn't meet consistency (in simulation)".into(),
                aggregations: vec![AggregationType::Sum],
            },
        ]
    }

    fn required_acks(&self) -> usize {
        match self.consistency_level {
            ConsistencyLevel::One => 1,
            ConsistencyLevel::Quorum => self.replication_factor / 2 + 1,
            ConsistencyLevel::All => self.replication_factor,
        }
    }

    fn simulated_lag(&self) -> f64 {
        if self.async_replication {
            // Simulate ~5ms lag per extra replica.
            (self.replication_factor as f64 - 1.0) * 5.0
        } else {
            0.0
        }
    }
}

impl Default for ReplicationBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for ReplicationBlock {
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
        if let Some(val) = params.get("replication_factor") {
            self.replication_factor = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("replication_factor must be an integer".into()))?
                as usize;
        }
        if let Some(val) = params.get("consistency_level") {
            let s = val
                .as_string()
                .ok_or_else(|| BlockError::InvalidParameter("consistency_level must be a string".into()))?;
            self.consistency_level = match s.to_lowercase().as_str() {
                "one" | "1" => ConsistencyLevel::One,
                "all" => ConsistencyLevel::All,
                _ => ConsistencyLevel::Quorum,
            };
        }
        if let Some(val) = params.get("async_replication") {
            self.async_replication = val
                .as_bool()
                .ok_or_else(|| BlockError::InvalidParameter("async_replication must be a boolean".into()))?;
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context.inputs.get("requests").cloned().unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        let required_acks = self.required_acks();
        let mut output_records = Vec::with_capacity(records.len());

        for record in records {
            self.writes_replicated += 1;

            // Simulate: all replicas ack (in this simulation we don't model failures).
            let acks = self.replication_factor;
            self.acks_received += acks;

            let meets_consistency = acks >= required_acks;
            if meets_consistency {
                self.consistency_met += 1;
            } else {
                self.consistency_violations += 1;
            }

            context.metrics.increment("writes_replicated");

            let mut out = record;
            let _ = out.insert("_replicas".into(), self.replication_factor);
            let _ = out.insert("_acks".into(), acks);
            let _ = out.insert("_consistency_met".into(), meets_consistency);
            output_records.push(out);
        }

        let lag = self.simulated_lag();
        context.metrics.record("replication_lag_ms", lag);
        context.metrics.record("consistency_met", self.consistency_met as f64);
        context.metrics.record("consistency_violations", self.consistency_violations as f64);

        let mut outputs = HashMap::new();
        outputs.insert("replicated".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("writes_replicated".into(), self.writes_replicated as f64);
        metrics_summary.insert("acks_received".into(), self.acks_received as f64);
        metrics_summary.insert("replication_lag_ms".into(), lag);
        metrics_summary.insert("consistency_met".into(), self.consistency_met as f64);
        metrics_summary.insert("consistency_violations".into(), self.consistency_violations as f64);

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("requests") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => ValidationResult::ok(),
                PortValue::None => ValidationResult::ok().with_warning("No writes to replicate"),
                _ => ValidationResult::error("requests port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("requests input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("replication_factor".into(), self.replication_factor);
        let _ = state.insert("writes_replicated".into(), self.writes_replicated);
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(r)) = state.get::<usize>("replication_factor") { self.replication_factor = r; }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_calculation() {
        let mut rep = ReplicationBlock::new();
        rep.replication_factor = 3;
        rep.consistency_level = ConsistencyLevel::Quorum;
        assert_eq!(rep.required_acks(), 2); // ⌊3/2⌋ + 1 = 2

        rep.replication_factor = 5;
        assert_eq!(rep.required_acks(), 3); // ⌊5/2⌋ + 1 = 3

        rep.consistency_level = ConsistencyLevel::All;
        assert_eq!(rep.required_acks(), 5);

        rep.consistency_level = ConsistencyLevel::One;
        assert_eq!(rep.required_acks(), 1);
    }

    #[test]
    fn test_async_lag() {
        let mut rep = ReplicationBlock::new();
        rep.replication_factor = 3;

        rep.async_replication = false;
        assert_eq!(rep.simulated_lag(), 0.0);

        rep.async_replication = true;
        assert_eq!(rep.simulated_lag(), 10.0); // (3-1) * 5ms
    }

    #[test]
    fn test_metadata() {
        let rep = ReplicationBlock::new();
        assert_eq!(rep.metadata().id, "replication");
        assert_eq!(rep.metadata().category, BlockCategory::Distribution);
    }
}
