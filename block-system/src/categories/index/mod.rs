//! Index block implementations
//!
//! Index blocks provide fast lookup structures over stored data.

pub mod btree;

pub use btree::BTreeIndexBlock;
