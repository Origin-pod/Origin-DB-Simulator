//! Constraint system for block requirements and guarantees
//!
//! This module defines the constraint system that allows blocks to declare
//! their requirements (what they need from the environment) and guarantees
//! (what they provide to other blocks).

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Forward declaration for Block trait (to avoid circular dependency)
// The actual Block trait is defined in the block module
use super::Block;

/// Constraint that a block requires from its environment
///
/// Constraints define what a block needs to function correctly.
/// The system must validate that all constraints are satisfied before
/// executing a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// The type of constraint being required
    pub constraint_type: ConstraintType,
    /// Human-readable description of why this constraint is needed
    pub description: String,
}

/// Types of constraints that blocks can require
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Requires another block to be present (by block ID)
    RequiresBlock(String),
    /// Requires a specific feature to be available
    RequiresFeature(String),
    /// Minimum memory required (in bytes)
    MinimumMemory(usize),
    /// Minimum disk space required (in bytes)
    MinimumDisk(usize),
    /// Block must be thread-safe
    ThreadSafe,
    /// Block requires atomic operations support
    AtomicOperations,
}

impl Constraint {
    /// Create a new constraint requiring another block
    pub fn requires_block(block_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            constraint_type: ConstraintType::RequiresBlock(block_id.into()),
            description: description.into(),
        }
    }

    /// Create a new constraint requiring a feature
    pub fn requires_feature(feature: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            constraint_type: ConstraintType::RequiresFeature(feature.into()),
            description: description.into(),
        }
    }

    /// Create a new minimum memory constraint
    pub fn minimum_memory(bytes: usize, description: impl Into<String>) -> Self {
        Self {
            constraint_type: ConstraintType::MinimumMemory(bytes),
            description: description.into(),
        }
    }

    /// Create a new minimum disk constraint
    pub fn minimum_disk(bytes: usize, description: impl Into<String>) -> Self {
        Self {
            constraint_type: ConstraintType::MinimumDisk(bytes),
            description: description.into(),
        }
    }

    /// Create a thread-safe constraint
    pub fn thread_safe(description: impl Into<String>) -> Self {
        Self {
            constraint_type: ConstraintType::ThreadSafe,
            description: description.into(),
        }
    }

    /// Create an atomic operations constraint
    pub fn atomic_operations(description: impl Into<String>) -> Self {
        Self {
            constraint_type: ConstraintType::AtomicOperations,
            description: description.into(),
        }
    }
}

/// Guarantee that a block provides
///
/// Guarantees define what properties a block maintains and what other blocks
/// can rely upon when using this block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guarantee {
    /// The type of guarantee being provided
    pub guarantee_type: GuaranteeType,
    /// Human-readable description of the guarantee
    pub description: String,
    /// Level of guarantee (strict or best-effort)
    pub level: GuaranteeLevel,
}

/// Types of guarantees that blocks can provide
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuaranteeType {
    /// Full ACID properties (Atomicity, Consistency, Isolation, Durability)
    Acid,
    /// Durability guarantee (data survives crashes)
    Durability,
    /// Consistency guarantee (maintains data invariants)
    Consistency,
    /// Isolation guarantee (concurrent operations don't interfere)
    Isolation,
    /// Atomicity guarantee (operations are all-or-nothing)
    Atomicity,
    /// Thread-safe guarantee (can be safely used from multiple threads)
    ThreadSafe,
    /// Serializable isolation level
    Serializable,
    /// Snapshot isolation level
    SnapshotIsolation,
}

/// Level of guarantee provided
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuaranteeLevel {
    /// Strict guarantee - always maintained
    Strict,
    /// Best-effort guarantee - maintained under normal conditions
    BestEffort,
}

impl Guarantee {
    /// Create a new guarantee
    pub fn new(
        guarantee_type: GuaranteeType,
        level: GuaranteeLevel,
        description: impl Into<String>,
    ) -> Self {
        Self {
            guarantee_type,
            description: description.into(),
            level,
        }
    }

    /// Create a strict guarantee
    pub fn strict(guarantee_type: GuaranteeType, description: impl Into<String>) -> Self {
        Self::new(guarantee_type, GuaranteeLevel::Strict, description)
    }

    /// Create a best-effort guarantee
    pub fn best_effort(guarantee_type: GuaranteeType, description: impl Into<String>) -> Self {
        Self::new(guarantee_type, GuaranteeLevel::BestEffort, description)
    }
}

/// Context for checking constraints
///
/// This context provides all the information needed to validate whether
/// a block's constraints can be satisfied in the current environment.
pub struct ConstraintContext {
    /// All blocks available in the system
    pub blocks: Vec<Arc<dyn Block>>,
    /// System configuration
    pub configuration: Configuration,
    /// Runtime environment information
    pub environment: Environment,
}

impl ConstraintContext {
    /// Create a new constraint context
    pub fn new(
        blocks: Vec<Arc<dyn Block>>,
        configuration: Configuration,
        environment: Environment,
    ) -> Self {
        Self {
            blocks,
            configuration,
            environment,
        }
    }

    /// Check if a constraint can be satisfied
    pub fn can_satisfy(&self, constraint: &Constraint) -> bool {
        match &constraint.constraint_type {
            ConstraintType::RequiresBlock(block_id) => {
                self.blocks.iter().any(|b| b.id().0.to_string() == *block_id)
            }
            ConstraintType::RequiresFeature(_feature) => {
                // Feature checking would be implemented based on actual features
                true // Placeholder
            }
            ConstraintType::MinimumMemory(required) => {
                if let Some(limit) = self.configuration.memory_limit {
                    limit >= *required
                } else {
                    self.environment.available_memory >= *required
                }
            }
            ConstraintType::MinimumDisk(required) => {
                if let Some(limit) = self.configuration.disk_limit {
                    limit >= *required
                } else {
                    self.environment.available_disk >= *required
                }
            }
            ConstraintType::ThreadSafe => {
                // Thread safety is a compile-time property in Rust
                true
            }
            ConstraintType::AtomicOperations => {
                // Atomic operations are always available in Rust
                true
            }
        }
    }

    /// Check all constraints for a block
    pub fn check_constraints(&self, constraints: &[Constraint]) -> ConstraintCheckResult {
        let mut result = ConstraintCheckResult {
            satisfied: true,
            failures: Vec::new(),
        };

        for constraint in constraints {
            if !self.can_satisfy(constraint) {
                result.satisfied = false;
                result.failures.push(ConstraintFailure {
                    constraint: constraint.clone(),
                    reason: format!("Constraint not satisfied: {}", constraint.description),
                });
            }
        }

        result
    }
}

/// Result of constraint checking
#[derive(Debug, Clone)]
pub struct ConstraintCheckResult {
    /// Whether all constraints are satisfied
    pub satisfied: bool,
    /// List of constraint failures
    pub failures: Vec<ConstraintFailure>,
}

/// Information about a constraint that failed
#[derive(Debug, Clone)]
pub struct ConstraintFailure {
    /// The constraint that failed
    pub constraint: Constraint,
    /// Reason for failure
    pub reason: String,
}

/// System configuration for constraint checking
#[derive(Debug, Clone)]
pub struct Configuration {
    /// Maximum memory limit (in bytes), if configured
    pub memory_limit: Option<usize>,
    /// Maximum disk limit (in bytes), if configured
    pub disk_limit: Option<usize>,
    /// Number of threads available for execution
    pub thread_count: usize,
}

impl Configuration {
    /// Create a new configuration
    pub fn new() -> Self {
        Self {
            memory_limit: None,
            disk_limit: None,
            thread_count: num_cpus(),
        }
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = Some(limit);
        self
    }

    /// Set disk limit
    pub fn with_disk_limit(mut self, limit: usize) -> Self {
        self.disk_limit = Some(limit);
        self
    }

    /// Set thread count
    pub fn with_thread_count(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime environment information
#[derive(Debug, Clone)]
pub struct Environment {
    /// Platform name (e.g., "linux", "windows", "macos")
    pub platform: String,
    /// Number of CPU cores available
    pub cpu_cores: usize,
    /// Available memory in bytes
    pub available_memory: usize,
    /// Available disk space in bytes
    pub available_disk: usize,
}

impl Environment {
    /// Create a new environment with system information
    pub fn new() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
            cpu_cores: num_cpus(),
            available_memory: estimate_available_memory(),
            available_disk: estimate_available_disk(),
        }
    }

    /// Create an environment with custom values (useful for testing)
    pub fn custom(
        platform: impl Into<String>,
        cpu_cores: usize,
        available_memory: usize,
        available_disk: usize,
    ) -> Self {
        Self {
            platform: platform.into(),
            cpu_cores,
            available_memory,
            available_disk,
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the number of CPU cores
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Estimate available memory (placeholder implementation)
fn estimate_available_memory() -> usize {
    // In a real implementation, this would query system information
    // For now, return a reasonable default (8GB)
    8 * 1024 * 1024 * 1024
}

/// Estimate available disk space (placeholder implementation)
fn estimate_available_disk() -> usize {
    // In a real implementation, this would query filesystem information
    // For now, return a reasonable default (100GB)
    100 * 1024 * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_creation() {
        let constraint = Constraint::requires_block("heap-file", "Needs heap file storage");
        assert!(matches!(
            constraint.constraint_type,
            ConstraintType::RequiresBlock(_)
        ));
        assert_eq!(constraint.description, "Needs heap file storage");
    }

    #[test]
    fn test_constraint_minimum_memory() {
        let constraint = Constraint::minimum_memory(1024 * 1024, "Needs 1MB memory");
        if let ConstraintType::MinimumMemory(bytes) = constraint.constraint_type {
            assert_eq!(bytes, 1024 * 1024);
        } else {
            panic!("Expected MinimumMemory constraint");
        }
    }

    #[test]
    fn test_guarantee_creation() {
        let guarantee = Guarantee::strict(GuaranteeType::Durability, "Data survives crashes");
        assert_eq!(guarantee.guarantee_type, GuaranteeType::Durability);
        assert_eq!(guarantee.level, GuaranteeLevel::Strict);
    }

    #[test]
    fn test_guarantee_best_effort() {
        let guarantee = Guarantee::best_effort(
            GuaranteeType::Consistency,
            "Maintains consistency under normal conditions",
        );
        assert_eq!(guarantee.guarantee_type, GuaranteeType::Consistency);
        assert_eq!(guarantee.level, GuaranteeLevel::BestEffort);
    }

    #[test]
    fn test_configuration_builder() {
        let config = Configuration::new()
            .with_memory_limit(1024 * 1024 * 1024)
            .with_disk_limit(10 * 1024 * 1024 * 1024)
            .with_thread_count(4);

        assert_eq!(config.memory_limit, Some(1024 * 1024 * 1024));
        assert_eq!(config.disk_limit, Some(10 * 1024 * 1024 * 1024));
        assert_eq!(config.thread_count, 4);
    }

    #[test]
    fn test_environment_custom() {
        let env = Environment::custom("linux", 8, 16 * 1024 * 1024 * 1024, 500 * 1024 * 1024 * 1024);
        assert_eq!(env.platform, "linux");
        assert_eq!(env.cpu_cores, 8);
        assert_eq!(env.available_memory, 16 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_constraint_serialization() {
        let constraint = Constraint::requires_feature("transactions", "Needs transaction support");
        let json = serde_json::to_string(&constraint).unwrap();
        let deserialized: Constraint = serde_json::from_str(&json).unwrap();

        match (&constraint.constraint_type, &deserialized.constraint_type) {
            (ConstraintType::RequiresFeature(f1), ConstraintType::RequiresFeature(f2)) => {
                assert_eq!(f1, f2);
            }
            _ => panic!("Constraint type mismatch"),
        }
    }

    #[test]
    fn test_guarantee_serialization() {
        let guarantee = Guarantee::strict(GuaranteeType::Atomicity, "All or nothing operations");
        let json = serde_json::to_string(&guarantee).unwrap();
        let deserialized: Guarantee = serde_json::from_str(&json).unwrap();

        assert_eq!(guarantee.guarantee_type, deserialized.guarantee_type);
        assert_eq!(guarantee.level, deserialized.level);
    }

    #[test]
    fn test_guarantee_types() {
        let types = vec![
            GuaranteeType::Acid,
            GuaranteeType::Durability,
            GuaranteeType::Consistency,
            GuaranteeType::Isolation,
            GuaranteeType::Atomicity,
            GuaranteeType::ThreadSafe,
            GuaranteeType::Serializable,
            GuaranteeType::SnapshotIsolation,
        ];

        for gt in types {
            let guarantee = Guarantee::strict(gt, "Test guarantee");
            assert_eq!(guarantee.guarantee_type, gt);
        }
    }

    #[test]
    fn test_constraint_types() {
        let constraints = vec![
            Constraint::requires_block("test", "test"),
            Constraint::requires_feature("test", "test"),
            Constraint::minimum_memory(1024, "test"),
            Constraint::minimum_disk(1024, "test"),
            Constraint::thread_safe("test"),
            Constraint::atomic_operations("test"),
        ];

        assert_eq!(constraints.len(), 6);
    }

    #[test]
    fn test_num_cpus() {
        let cores = num_cpus();
        assert!(cores >= 1);
    }
}
