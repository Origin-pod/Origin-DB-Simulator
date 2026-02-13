//! Optimization block implementations
//!
//! Blocks that help query planners and execution engines make better decisions.

pub mod bloom_filter;
pub mod statistics_collector;

pub use bloom_filter::BloomFilterBlock;
pub use statistics_collector::StatisticsCollectorBlock;
