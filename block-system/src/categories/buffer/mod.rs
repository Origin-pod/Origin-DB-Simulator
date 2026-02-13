//! Buffer management block implementations
//!
//! Buffer blocks cache pages in memory to reduce storage I/O.

pub mod lru_buffer;
pub mod clock_buffer;

pub use lru_buffer::LRUBufferBlock;
pub use clock_buffer::ClockBufferBlock;
