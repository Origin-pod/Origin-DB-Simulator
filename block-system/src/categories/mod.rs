//! Block categories and classification
//!
//! This module handles block categorization and provides category-specific
//! abstractions for different types of blocks.

use serde::{Deserialize, Serialize};

/// Block category enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockCategory {
    /// Data source blocks (e.g., CSV, Database, API)
    DataSource,
    /// Data transformation blocks (e.g., Filter, Map, Join)
    Transformation,
    /// Data aggregation blocks (e.g., Group By, Count)
    Aggregation,
    /// Data output blocks (e.g., File, Database, API)
    Output,
    /// Control flow blocks (e.g., Conditional, Loop)
    ControlFlow,
    /// Custom user-defined blocks
    Custom(String),
}

impl BlockCategory {
    /// Get a human-readable name for the category
    pub fn display_name(&self) -> &str {
        match self {
            BlockCategory::DataSource => "Data Source",
            BlockCategory::Transformation => "Transformation",
            BlockCategory::Aggregation => "Aggregation",
            BlockCategory::Output => "Output",
            BlockCategory::ControlFlow => "Control Flow",
            BlockCategory::Custom(name) => name,
        }
    }
}

impl std::fmt::Display for BlockCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
