//! Workload generator
//!
//! Generates streams of database operations (INSERT, SELECT, UPDATE, DELETE)
//! with configurable weights and key distributions (uniform, zipfian, latest).
//! Produces `Vec<Record>` that can feed into entry-point blocks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::port::Record;

// ── Configuration types ─────────────────────────────────────────────────────

/// Top-level workload configuration — mirrors frontend `WorkloadConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadConfig {
    /// Operation mix (type + weight).
    pub operations: Vec<OperationConfig>,
    /// Key distribution strategy.
    pub distribution: Distribution,
    /// Total number of operations to generate.
    pub total_ops: usize,
    /// Random seed for reproducibility (0 = non-deterministic).
    pub seed: u64,
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        Self {
            operations: vec![
                OperationConfig {
                    op_type: OperationType::Insert,
                    weight: 50,
                },
                OperationConfig {
                    op_type: OperationType::Select,
                    weight: 30,
                },
                OperationConfig {
                    op_type: OperationType::Update,
                    weight: 15,
                },
                OperationConfig {
                    op_type: OperationType::Delete,
                    weight: 5,
                },
            ],
            distribution: Distribution::Uniform,
            total_ops: 1000,
            seed: 0,
        }
    }
}

/// A single operation type and its relative weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationConfig {
    pub op_type: OperationType,
    /// Relative weight (higher = more frequent).
    pub weight: u32,
}

/// Supported operation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    Insert,
    Select,
    Update,
    Delete,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Insert => write!(f, "INSERT"),
            OperationType::Select => write!(f, "SELECT"),
            OperationType::Update => write!(f, "UPDATE"),
            OperationType::Delete => write!(f, "DELETE"),
        }
    }
}

/// Key distribution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Distribution {
    /// Each key equally likely.
    Uniform,
    /// Popular keys accessed much more often (models real-world skew).
    Zipfian,
    /// Most recent keys accessed most often.
    Latest,
}

// ── Generated operation ─────────────────────────────────────────────────────

/// A single generated operation.
#[derive(Debug, Clone)]
pub struct Operation {
    /// Sequence number (0-based).
    pub seq: usize,
    /// The operation type.
    pub op_type: OperationType,
    /// Target key id.
    pub key: usize,
}

impl Operation {
    /// Convert to a `Record` suitable for block input ports.
    pub fn to_record(&self) -> Record {
        let mut r = Record::new();
        r.insert("_op_type".into(), self.op_type.to_string()).ok();
        r.insert("_op_seq".into(), self.seq as i64).ok();
        r.insert("id".into(), self.key as i64).ok();
        r.insert("name".into(), format!("user_{}", self.key)).ok();
        r.insert("score".into(), ((self.key * 7) % 100) as f64).ok();
        r
    }
}

// ── Simple deterministic PRNG (xorshift64) ──────────────────────────────────

/// Minimal xorshift64 PRNG — no external dependency needed.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid zero state.
        Self {
            state: if seed == 0 {
                0x853c_49e6_748f_ea9b
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform random in [0, n).
    fn next_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// ── Generator ───────────────────────────────────────────────────────────────

/// Generates a workload of operations according to the config.
pub struct WorkloadGenerator;

impl WorkloadGenerator {
    /// Generate a sequence of operations from the config.
    pub fn generate(config: &WorkloadConfig) -> Vec<Operation> {
        let mut rng = Rng::new(config.seed);

        // Build weighted operation type table.
        let op_table = Self::build_op_table(&config.operations);
        if op_table.is_empty() {
            return Vec::new();
        }

        // Key space size: for inserts we keep growing; for others we pick
        // from existing keys.  We'll track a "next_key" counter.
        let mut next_key: usize = 0;
        let mut ops = Vec::with_capacity(config.total_ops);

        for seq in 0..config.total_ops {
            // Pick operation type.
            let op_idx = rng.next_usize(op_table.len());
            let op_type = op_table[op_idx];

            // Pick key.
            let key = match op_type {
                OperationType::Insert => {
                    let k = next_key;
                    next_key += 1;
                    k
                }
                _ => {
                    if next_key == 0 {
                        // No keys inserted yet — force an insert.
                        let k = next_key;
                        next_key += 1;
                        ops.push(Operation {
                            seq,
                            op_type: OperationType::Insert,
                            key: k,
                        });
                        continue;
                    }
                    Self::pick_key(&mut rng, next_key, config.distribution)
                }
            };

            ops.push(Operation { seq, op_type, key });
        }

        ops
    }

    /// Convert generated operations into Records for block consumption.
    pub fn generate_records(config: &WorkloadConfig) -> Vec<Record> {
        Self::generate(config)
            .iter()
            .map(|op| op.to_record())
            .collect()
    }

    /// Workload summary statistics.
    pub fn summarize(ops: &[Operation]) -> HashMap<OperationType, usize> {
        let mut counts = HashMap::new();
        for op in ops {
            *counts.entry(op.op_type).or_insert(0) += 1;
        }
        counts
    }

    // ── Internal ────────────────────────────────────────────────────────

    /// Build a flat table where each op type appears proportional to weight.
    fn build_op_table(configs: &[OperationConfig]) -> Vec<OperationType> {
        // Find GCD of all weights to keep the table small.
        let total: u32 = configs.iter().map(|c| c.weight).sum();
        if total == 0 {
            return Vec::new();
        }

        let mut table = Vec::new();
        for cfg in configs {
            if cfg.weight > 0 {
                // Each weight unit contributes one entry.
                for _ in 0..cfg.weight {
                    table.push(cfg.op_type);
                }
            }
        }
        table
    }

    /// Pick a key from [0, key_count) using the given distribution.
    fn pick_key(rng: &mut Rng, key_count: usize, dist: Distribution) -> usize {
        match dist {
            Distribution::Uniform => rng.next_usize(key_count),
            Distribution::Zipfian => Self::zipfian_key(rng, key_count),
            Distribution::Latest => Self::latest_key(rng, key_count),
        }
    }

    /// Zipfian: heavily skewed toward low keys.
    /// Approximation: take the floor of `key_count * (1 - random^2)` inverted
    /// so key 0 is most popular.
    fn zipfian_key(rng: &mut Rng, key_count: usize) -> usize {
        // Generate a number in (0, 1).
        let u = (rng.next_u64() as f64) / (u64::MAX as f64);
        // Power-law bias toward 0.
        let biased = u * u;
        let key = (biased * key_count as f64) as usize;
        key.min(key_count - 1)
    }

    /// Latest: most recent keys are most popular.
    /// Exponential decay from the highest key.
    fn latest_key(rng: &mut Rng, key_count: usize) -> usize {
        let u = (rng.next_u64() as f64) / (u64::MAX as f64);
        // Exponential bias toward the latest (highest) key.
        let offset = (u * u * key_count as f64) as usize;
        key_count.saturating_sub(1).saturating_sub(offset)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_generates_ops() {
        let config = WorkloadConfig {
            seed: 42,
            ..Default::default()
        };
        let ops = WorkloadGenerator::generate(&config);
        assert_eq!(ops.len(), 1000);
    }

    #[test]
    fn test_deterministic_with_seed() {
        let config = WorkloadConfig {
            seed: 123,
            total_ops: 100,
            ..Default::default()
        };
        let ops1 = WorkloadGenerator::generate(&config);
        let ops2 = WorkloadGenerator::generate(&config);

        for (a, b) in ops1.iter().zip(ops2.iter()) {
            assert_eq!(a.op_type, b.op_type);
            assert_eq!(a.key, b.key);
        }
    }

    #[test]
    fn test_insert_only_workload() {
        let config = WorkloadConfig {
            operations: vec![OperationConfig {
                op_type: OperationType::Insert,
                weight: 100,
            }],
            total_ops: 50,
            seed: 1,
            distribution: Distribution::Uniform,
        };
        let ops = WorkloadGenerator::generate(&config);
        assert_eq!(ops.len(), 50);
        assert!(ops.iter().all(|op| op.op_type == OperationType::Insert));

        // Keys should be sequential.
        for (i, op) in ops.iter().enumerate() {
            assert_eq!(op.key, i);
        }
    }

    #[test]
    fn test_select_only_forces_initial_insert() {
        let config = WorkloadConfig {
            operations: vec![OperationConfig {
                op_type: OperationType::Select,
                weight: 100,
            }],
            total_ops: 10,
            seed: 1,
            distribution: Distribution::Uniform,
        };
        let ops = WorkloadGenerator::generate(&config);
        // First op should be forced INSERT because no keys exist.
        assert_eq!(ops[0].op_type, OperationType::Insert);
    }

    #[test]
    fn test_operation_weights_respected() {
        let config = WorkloadConfig {
            operations: vec![
                OperationConfig {
                    op_type: OperationType::Insert,
                    weight: 100,
                },
                OperationConfig {
                    op_type: OperationType::Select,
                    weight: 0,
                },
            ],
            total_ops: 200,
            seed: 42,
            distribution: Distribution::Uniform,
        };
        let ops = WorkloadGenerator::generate(&config);
        let summary = WorkloadGenerator::summarize(&ops);
        assert_eq!(*summary.get(&OperationType::Insert).unwrap_or(&0), 200);
        assert_eq!(*summary.get(&OperationType::Select).unwrap_or(&0), 0);
    }

    #[test]
    fn test_to_record_has_expected_fields() {
        let op = Operation {
            seq: 5,
            op_type: OperationType::Insert,
            key: 42,
        };
        let rec = op.to_record();
        assert!(rec.data.contains_key("_op_type"));
        assert!(rec.data.contains_key("_op_seq"));
        assert!(rec.data.contains_key("id"));
        assert!(rec.data.contains_key("name"));
        assert!(rec.data.contains_key("score"));

        assert_eq!(rec.get::<i64>("id").unwrap().unwrap(), 42);
        assert_eq!(rec.get::<String>("_op_type").unwrap().unwrap(), "INSERT");
    }

    #[test]
    fn test_generate_records() {
        let config = WorkloadConfig {
            total_ops: 20,
            seed: 7,
            ..Default::default()
        };
        let records = WorkloadGenerator::generate_records(&config);
        assert_eq!(records.len(), 20);
        for r in &records {
            assert!(r.data.contains_key("id"));
        }
    }

    #[test]
    fn test_zipfian_distribution_skew() {
        let config = WorkloadConfig {
            operations: vec![
                OperationConfig {
                    op_type: OperationType::Insert,
                    weight: 10,
                },
                OperationConfig {
                    op_type: OperationType::Select,
                    weight: 90,
                },
            ],
            total_ops: 10_000,
            seed: 42,
            distribution: Distribution::Zipfian,
        };
        let ops = WorkloadGenerator::generate(&config);

        // Count selects targeting key 0 vs other keys.
        let selects: Vec<_> = ops
            .iter()
            .filter(|op| op.op_type == OperationType::Select)
            .collect();

        let key_0_count = selects.iter().filter(|op| op.key == 0).count();

        // Zipfian should make key 0 much more popular than uniform 1/N.
        // With N ~1000 inserts, uniform would give ~selects.len()/1000 ≈ 9.
        // Zipfian should give significantly more.
        assert!(
            key_0_count > 50,
            "Zipfian should heavily favor key 0, got {} / {}",
            key_0_count,
            selects.len()
        );
    }

    #[test]
    fn test_latest_distribution_skew() {
        // Compare latest vs uniform: latest should have higher average key
        // relative to uniform when both use the same seed and config.
        let base = WorkloadConfig {
            operations: vec![
                OperationConfig {
                    op_type: OperationType::Insert,
                    weight: 10,
                },
                OperationConfig {
                    op_type: OperationType::Select,
                    weight: 90,
                },
            ],
            total_ops: 5_000,
            seed: 42,
            distribution: Distribution::Uniform,
        };

        let uniform_ops = WorkloadGenerator::generate(&base);
        let uniform_select_avg = {
            let selects: Vec<_> = uniform_ops.iter().filter(|o| o.op_type == OperationType::Select).collect();
            selects.iter().map(|o| o.key as f64).sum::<f64>() / selects.len() as f64
        };

        let latest_config = WorkloadConfig {
            distribution: Distribution::Latest,
            ..base
        };
        let latest_ops = WorkloadGenerator::generate(&latest_config);
        let latest_select_avg = {
            let selects: Vec<_> = latest_ops.iter().filter(|o| o.op_type == OperationType::Select).collect();
            selects.iter().map(|o| o.key as f64).sum::<f64>() / selects.len() as f64
        };

        // Latest distribution should have a higher average select key than uniform
        // because it biases toward the most recently inserted keys.
        assert!(
            latest_select_avg > uniform_select_avg,
            "Latest avg {:.1} should exceed uniform avg {:.1}",
            latest_select_avg,
            uniform_select_avg
        );
    }

    #[test]
    fn test_empty_operations_returns_empty() {
        let config = WorkloadConfig {
            operations: vec![],
            total_ops: 100,
            seed: 1,
            distribution: Distribution::Uniform,
        };
        let ops = WorkloadGenerator::generate(&config);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_summarize() {
        let ops = vec![
            Operation { seq: 0, op_type: OperationType::Insert, key: 0 },
            Operation { seq: 1, op_type: OperationType::Insert, key: 1 },
            Operation { seq: 2, op_type: OperationType::Select, key: 0 },
            Operation { seq: 3, op_type: OperationType::Delete, key: 0 },
        ];
        let summary = WorkloadGenerator::summarize(&ops);
        assert_eq!(*summary.get(&OperationType::Insert).unwrap(), 2);
        assert_eq!(*summary.get(&OperationType::Select).unwrap(), 1);
        assert_eq!(*summary.get(&OperationType::Delete).unwrap(), 1);
        assert_eq!(summary.get(&OperationType::Update), None);
    }
}
