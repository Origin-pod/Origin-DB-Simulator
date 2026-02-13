//! Block categories and implementations
//!
//! This module provides block categorization as well as concrete
//! block implementations organized by category.

pub mod storage;
pub mod index;
pub mod buffer;
pub mod execution;
pub mod concurrency;
pub mod transaction;
pub mod optimization;
pub mod partitioning;
pub mod distribution;
pub mod compression;

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

// ---------------------------------------------------------------------------
// Shared types used across block categories
// ---------------------------------------------------------------------------

/// Identifies a specific record within a heap file.
///
/// A TupleId (also called a Record ID or RID) combines a page number
/// with a slot number to locate a record on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TupleId {
    /// The page containing this record
    pub page_id: usize,
    /// The slot within the page
    pub slot_id: usize,
}

impl TupleId {
    pub fn new(page_id: usize, slot_id: usize) -> Self {
        Self { page_id, slot_id }
    }
}

impl std::fmt::Display for TupleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.page_id, self.slot_id)
    }
}
