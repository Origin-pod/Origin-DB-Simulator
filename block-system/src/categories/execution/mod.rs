//! Execution block implementations
//!
//! Execution blocks implement query processing operators like scans, joins, and filters.

pub mod sequential_scan;
pub mod index_scan;
pub mod filter;
pub mod sort;
pub mod hash_join;

pub use sequential_scan::SequentialScanBlock;
pub use index_scan::IndexScanBlock;
pub use filter::FilterBlock;
pub use sort::SortBlock;
pub use hash_join::HashJoinBlock;
