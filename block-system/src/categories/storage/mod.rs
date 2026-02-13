//! Storage block implementations
//!
//! Storage blocks manage how data is physically organized and stored.

pub mod heap_file;
pub mod lsm_tree;

pub use heap_file::HeapFileBlock;
pub use lsm_tree::LSMTreeBlock;
