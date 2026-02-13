//! Concurrency control block implementations
//!
//! Concurrency blocks manage how multiple transactions access shared data safely.

pub mod row_lock;

pub use row_lock::RowLockBlock;
