//! Index block implementations
//!
//! Index blocks provide fast lookup structures over stored data.

pub mod btree;
pub mod hash_index;

pub use btree::BTreeIndexBlock;
pub use hash_index::HashIndexBlock;
