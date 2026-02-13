//! Compression block implementations
//!
//! Blocks that compress data to reduce storage and I/O costs.

pub mod dictionary_encoding;

pub use dictionary_encoding::DictionaryEncodingBlock;
