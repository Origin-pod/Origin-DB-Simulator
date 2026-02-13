//! Execution block implementations
//!
//! Execution blocks implement query processing operators like scans, joins, and filters.

pub mod sequential_scan;
pub mod index_scan;

pub use sequential_scan::SequentialScanBlock;
pub use index_scan::IndexScanBlock;
