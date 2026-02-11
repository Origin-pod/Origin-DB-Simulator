//! Storage block implementations
//!
//! Storage blocks manage how data is physically organized and stored.

pub mod heap_file;

pub use heap_file::HeapFileBlock;
