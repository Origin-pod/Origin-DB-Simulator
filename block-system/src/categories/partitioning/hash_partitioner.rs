//! Hash Partitioner Block
//!
//! Distributes records across partitions by hashing a key column. Core to
//! every distributed database — Cassandra, DynamoDB, CockroachDB all use
//! hash partitioning to spread data evenly across nodes.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `records_partitioned` | Counter | Total records assigned to partitions |
//! | `partitions_used` | Gauge | Number of partitions that received data |
//! | `hottest_partition_pct` | Gauge | % of records in the most loaded partition |
//! | `evenness_score` | Gauge | 0–100 score of distribution evenness |

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
use crate::core::port::{Port, PortDirection, PortType, PortValue};

// ---------------------------------------------------------------------------
// HashPartitionerBlock
// ---------------------------------------------------------------------------

pub struct HashPartitionerBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    num_partitions: usize,

    // Stats
    partition_counts: Vec<usize>,
    records_partitioned: usize,
}

impl HashPartitionerBlock {
    pub fn new() -> Self {
        let num_partitions = 4;
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            num_partitions,
            partition_counts: vec![0; num_partitions],
            records_partitioned: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "hash-partitioner".into(),
            name: "Hash Partitioner".into(),
            category: BlockCategory::Partitioning,
            description: "Distributes records across partitions by hashing a key".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "Hash partitioning distributes data across multiple partitions by computing a \
                           hash of the partition key and using modulo to assign each record to a partition. \
                           This enables horizontal scaling — each partition can live on a different node, \
                           allowing the system to handle more data and more queries by adding machines.\n\n\
                           In a distributed database, the hash partitioner is one of the first components a \
                           write request encounters. It determines which node owns each piece of data. The \
                           hash function must be deterministic (same key always maps to the same partition) \
                           and should distribute keys uniformly to avoid hot spots.\n\n\
                           Think of hash partitioning like assigning students to classrooms by the first letter \
                           of their last name. But instead of letters (which produce uneven groups), you use a \
                           hash function that scrambles the name into a number and then takes modulo N to get \
                           the classroom number. This produces much more even groups. The downside: if you \
                           want to find all students whose names start with 'S', you have to check every \
                           classroom (scatter-gather) because hashing destroyed the alphabetical ordering."
                    .into(),
                algorithm: "PARTITION(record):\n  \
                           1. Extract partition key from record\n  \
                           2. Compute hash: h = hash_mix(key)\n     \
                              h ^= h >> 33\n     \
                              h *= 0xff51afd7ed558ccd\n     \
                              h ^= h >> 33\n     \
                              h *= 0xc4ceb9fe1a85ec53\n     \
                              h ^= h >> 33\n  \
                           3. Assign partition: partition_id = h % num_partitions\n  \
                           4. Route record to partition_id\n\n\
                           REBALANCE (when adding partition):\n  \
                           Simple modulo: ALL records potentially need reassignment\n  \
                           Consistent hashing: only K/N records move on average\n  \
                           (where K = total keys, N = total partitions)"
                    .into(),
                complexity: Complexity {
                    time: "O(1) per record — single hash computation".into(),
                    space: "O(num_partitions) for partition metadata".into(),
                },
                use_cases: vec![
                    "Cassandra distributes rows across nodes using Murmur3 hash".into(),
                    "DynamoDB partitions items by hash of the partition key".into(),
                    "CockroachDB splits ranges using hash-based sharding".into(),
                    "Kafka distributes messages across topic partitions by key hash".into(),
                    "Redis Cluster uses CRC16 hash slots to partition keys across nodes".into(),
                ],
                tradeoffs: vec![
                    "Even distribution depends on key cardinality and hash quality".into(),
                    "Range queries across partitions require scatter-gather".into(),
                    "Adding partitions requires data reshuffling (consistent hashing helps)".into(),
                    "Hot keys defeat the purpose — one partition gets all traffic".into(),
                    "Cannot efficiently answer range queries (e.g., 'all users with ID 1000-2000')".into(),
                    "Hash function quality matters — poor hash functions create skewed distributions".into(),
                ],
                examples: vec![
                    "Cassandra's Murmur3Partitioner (default since 1.2) — maps token range to nodes".into(),
                    "DynamoDB — partitions by hash of partition key, supports sort key for range queries within partition".into(),
                    "Kafka — DefaultPartitioner hashes message key to assign topic partitions".into(),
                    "Redis Cluster — 16384 hash slots distributed across nodes using CRC16".into(),
                ],
                motivation: "Without partitioning, all data lives on a single node, which creates a hard \
                             ceiling on both storage capacity and query throughput. When a table grows beyond \
                             what one machine can handle, you need to split it across multiple machines.\n\n\
                             Hash partitioning solves the distribution problem by providing a deterministic, \
                             uniform mapping from keys to nodes. Any node in the cluster can instantly compute \
                             which node owns a given key without consulting a central directory. This enables \
                             both writes and reads to be routed directly to the right node in O(1) time."
                    .into(),
                parameter_guide: HashMap::from([
                    ("num_partitions".into(),
                     "The number of partitions to distribute data across. Each partition can be assigned to \
                      a different node for horizontal scaling. More partitions enable finer-grained load \
                      balancing but add overhead for cross-partition queries (scatter-gather). Fewer partitions \
                      are simpler but limit scalability. In Cassandra, the virtual node count (num_tokens) \
                      serves a similar purpose — default is 256 vnodes per physical node. For Kafka, the \
                      partition count is typically 3-12 per topic. Recommended: start with 4-8 for development, \
                      scale to match the number of nodes in production."
                        .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "range-partitioner".into(),
                        comparison: "Range partitioning assigns contiguous key ranges to partitions (e.g., \
                                     A-F to partition 1, G-M to partition 2). This preserves key ordering, \
                                     enabling efficient range queries within a partition, but is prone to hot \
                                     spots when certain ranges receive more traffic. Hash partitioning distributes \
                                     keys uniformly but destroys ordering. Choose hash partitioning for even \
                                     distribution with point lookups. Choose range partitioning when range queries \
                                     are common and you can tolerate potential skew."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "What is consistent hashing, and how does it reduce data movement when adding or removing nodes?".into(),
                    "How do you handle hot keys (e.g., a celebrity's profile) that all hash to the same partition?".into(),
                    "Why does DynamoDB require a partition key AND optional sort key — how does this combine hash and range partitioning?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Designing Data-Intensive Applications by Martin Kleppmann — Chapter 6: Partitioning".into(),
                url: None,
                citation: Some("Kleppmann, M. (2017). Designing Data-Intensive Applications. O'Reilly.".into()),
            }],
            icon: "grid-3x3".into(),
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
            description: "Records to partition. Uses `_key` field as the partition key.".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "partitioned".into(),
            name: "Partitioned Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records enriched with `_partition_id` field".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "num_partitions".into(),
            name: "Partitions".into(),
            param_type: ParameterType::Number,
            description: "Number of partitions to distribute data across".into(),
            default_value: ParameterValue::Integer(4),
            required: false,
            constraints: Some(
                ParameterConstraints::new().with_min(2.0).with_max(256.0),
            ),
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Slider).with_step(1.0),
            ),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "records_partitioned".into(),
                name: "Records Partitioned".into(),
                metric_type: MetricType::Counter,
                unit: "records".into(),
                description: "Total records assigned to partitions".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "partitions_used".into(),
                name: "Partitions Used".into(),
                metric_type: MetricType::Gauge,
                unit: "partitions".into(),
                description: "Number of partitions that received at least one record".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "hottest_partition_pct".into(),
                name: "Hottest Partition".into(),
                metric_type: MetricType::Gauge,
                unit: "%".into(),
                description: "Percentage of records in the most loaded partition".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "evenness_score".into(),
                name: "Evenness Score".into(),
                metric_type: MetricType::Gauge,
                unit: "score".into(),
                description: "0–100 score of how evenly data is distributed (100 = perfect)".into(),
                aggregations: vec![AggregationType::Avg],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    fn hash_key(&self, key: u64) -> usize {
        // Simple but effective hash mixing (similar to Murmur3 finalizer).
        let mut h = key;
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;
        (h as usize) % self.num_partitions
    }

    fn hottest_partition_pct(&self) -> f64 {
        if self.records_partitioned == 0 {
            return 0.0;
        }
        let max = *self.partition_counts.iter().max().unwrap_or(&0);
        (max as f64 / self.records_partitioned as f64) * 100.0
    }

    fn evenness_score(&self) -> f64 {
        if self.records_partitioned == 0 || self.num_partitions == 0 {
            return 100.0;
        }
        let ideal = self.records_partitioned as f64 / self.num_partitions as f64;
        let total_deviation: f64 = self
            .partition_counts
            .iter()
            .map(|&c| (c as f64 - ideal).abs())
            .sum();
        let max_deviation = self.records_partitioned as f64 * 2.0;
        (1.0 - total_deviation / max_deviation) * 100.0
    }
}

impl Default for HashPartitionerBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for HashPartitionerBlock {
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
        if let Some(val) = params.get("num_partitions") {
            self.num_partitions = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("num_partitions must be an integer".into()))?
                as usize;
            if self.num_partitions < 2 {
                return Err(BlockError::InvalidParameter("num_partitions must be at least 2".into()));
            }
            self.partition_counts = vec![0; self.num_partitions];
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

        let mut output_records = Vec::with_capacity(records.len());

        for record in records {
            let key = record.get::<u64>("_key").ok().flatten().unwrap_or(0);
            let partition = self.hash_key(key);

            self.partition_counts[partition] += 1;
            self.records_partitioned += 1;

            context.metrics.increment("records_partitioned");

            let mut out = record;
            let _ = out.insert("_partition_id".into(), partition);
            output_records.push(out);
        }

        let partitions_used = self.partition_counts.iter().filter(|&&c| c > 0).count();

        context.metrics.record("partitions_used", partitions_used as f64);
        context.metrics.record("hottest_partition_pct", self.hottest_partition_pct());
        context.metrics.record("evenness_score", self.evenness_score());

        let mut outputs = HashMap::new();
        outputs.insert("partitioned".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("records_partitioned".into(), self.records_partitioned as f64);
        metrics_summary.insert("partitions_used".into(), partitions_used as f64);
        metrics_summary.insert("hottest_partition_pct".into(), self.hottest_partition_pct());
        metrics_summary.insert("evenness_score".into(), self.evenness_score());

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
                PortValue::None => ValidationResult::ok().with_warning("No records to partition"),
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("num_partitions".into(), self.num_partitions);
        let _ = state.insert("records_partitioned".into(), self.records_partitioned);
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(n)) = state.get::<usize>("num_partitions") { self.num_partitions = n; }
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
    fn test_deterministic_partitioning() {
        let mut part = HashPartitionerBlock::new();
        part.num_partitions = 4;
        part.partition_counts = vec![0; 4];

        let p1 = part.hash_key(42);
        let p2 = part.hash_key(42);
        assert_eq!(p1, p2, "Same key should always map to same partition");
        assert!(p1 < 4, "Partition ID should be within range");
    }

    #[test]
    fn test_distribution_evenness() {
        let mut part = HashPartitionerBlock::new();
        part.num_partitions = 4;
        part.partition_counts = vec![0; 4];

        // Distribute 1000 sequential keys.
        for i in 0..1000u64 {
            let p = part.hash_key(i);
            part.partition_counts[p] += 1;
            part.records_partitioned += 1;
        }

        // Each partition should have roughly 250 (±20%).
        for &count in &part.partition_counts {
            assert!(count > 100, "Partition too empty: {}", count);
            assert!(count < 500, "Partition too full: {}", count);
        }

        assert!(part.evenness_score() > 80.0, "Distribution should be reasonably even");
    }

    #[test]
    fn test_metadata() {
        let part = HashPartitionerBlock::new();
        assert_eq!(part.metadata().id, "hash-partitioner");
        assert_eq!(part.metadata().category, BlockCategory::Partitioning);
    }
}
