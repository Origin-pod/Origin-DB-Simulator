//! Example block implementations
//!
//! This module provides complete, working examples of blocks to help understand
//! how the block system works in practice. These examples are fully functional
//! and demonstrate best practices for implementing blocks.

#[cfg(test)]
mod tests {
    use crate::core::{Block, BlockId, BlockMetadata};
    use crate::categories::BlockCategory;
    use crate::runtime::BlockRuntime;
    use std::sync::Arc;

    // ============================================================================
    // Example 1: Simple Counter Block
    // ============================================================================

    /// A simple counter block that counts operations
    ///
    /// This is the simplest possible block implementation. It demonstrates:
    /// - Basic Block trait implementation
    /// - Metadata definition
    /// - State management
    /// - Validation
    #[derive(Clone)]
    struct CounterBlock {
        id: BlockId,
        metadata: BlockMetadata,
        count: u64,
    }

    impl CounterBlock {
        fn new() -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: "Counter".to_string(),
                    description: "Counts the number of operations performed".to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec![
                        "counter".to_string(),
                        "metrics".to_string(),
                        "category:Transformation".to_string(),
                    ],
                },
                count: 0,
            }
        }

        /// Increment the counter
        fn increment(&mut self) {
            self.count += 1;
        }

        /// Get the current count
        fn get_count(&self) -> u64 {
            self.count
        }

        /// Reset the counter
        fn reset(&mut self) {
            self.count = 0;
        }
    }

    impl Block for CounterBlock {
        fn id(&self) -> BlockId {
            self.id
        }

        fn metadata(&self) -> &BlockMetadata {
            &self.metadata
        }

        fn validate(&self) -> Result<(), String> {
            if self.metadata.name.is_empty() {
                return Err("Block name cannot be empty".to_string());
            }
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn Block> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_counter_block() {
        let mut counter = CounterBlock::new();

        // Initial count is 0
        assert_eq!(counter.get_count(), 0);

        // Increment the counter
        counter.increment();
        counter.increment();
        counter.increment();
        assert_eq!(counter.get_count(), 3);

        // Reset works
        counter.reset();
        assert_eq!(counter.get_count(), 0);

        // Validation passes
        assert!(counter.validate().is_ok());
    }

    // ============================================================================
    // Example 2: Filter Block
    // ============================================================================

    /// A filter block that filters data based on a predicate
    ///
    /// This demonstrates:
    /// - Configurable blocks (with filter criteria)
    /// - More complex internal state
    /// - Tagged categorization
    #[derive(Clone)]
    struct FilterBlock {
        id: BlockId,
        metadata: BlockMetadata,
        threshold: i32,
        inverted: bool,
    }

    impl FilterBlock {
        fn new(threshold: i32, inverted: bool) -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: "Filter".to_string(),
                    description: format!(
                        "Filters values {} threshold of {}",
                        if inverted { "below" } else { "above" },
                        threshold
                    ),
                    version: "1.0.0".to_string(),
                    tags: vec![
                        "filter".to_string(),
                        "transformation".to_string(),
                        "category:Transformation".to_string(),
                    ],
                },
                threshold,
                inverted,
            }
        }

        /// Check if a value passes the filter
        fn passes(&self, value: i32) -> bool {
            if self.inverted {
                value < self.threshold
            } else {
                value >= self.threshold
            }
        }

        /// Filter a collection of values
        fn filter_values(&self, values: &[i32]) -> Vec<i32> {
            values.iter().filter(|&&v| self.passes(v)).copied().collect()
        }
    }

    impl Block for FilterBlock {
        fn id(&self) -> BlockId {
            self.id
        }

        fn metadata(&self) -> &BlockMetadata {
            &self.metadata
        }

        fn validate(&self) -> Result<(), String> {
            // No specific validation needed for this block
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn Block> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_filter_block() {
        // Create a filter that keeps values >= 50
        let filter = FilterBlock::new(50, false);

        assert!(filter.passes(50));
        assert!(filter.passes(100));
        assert!(!filter.passes(49));
        assert!(!filter.passes(0));

        // Test filtering a collection
        let values = vec![10, 20, 50, 75, 100, 25];
        let filtered = filter.filter_values(&values);
        assert_eq!(filtered, vec![50, 75, 100]);
    }

    #[test]
    fn test_filter_block_inverted() {
        // Create an inverted filter that keeps values < 50
        let filter = FilterBlock::new(50, true);

        assert!(!filter.passes(50));
        assert!(!filter.passes(100));
        assert!(filter.passes(49));
        assert!(filter.passes(0));

        // Test filtering a collection
        let values = vec![10, 20, 50, 75, 100, 25];
        let filtered = filter.filter_values(&values);
        assert_eq!(filtered, vec![10, 20, 25]);
    }

    // ============================================================================
    // Example 3: Buffer Block
    // ============================================================================

    /// A buffer block that stores data temporarily
    ///
    /// This demonstrates:
    /// - Capacity management
    /// - Push/pop operations
    /// - Full/empty state checks
    /// - More complex validation logic
    #[derive(Clone)]
    struct BufferBlock {
        id: BlockId,
        metadata: BlockMetadata,
        capacity: usize,
        data: Vec<String>,
    }

    impl BufferBlock {
        fn new(capacity: usize) -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: "Buffer".to_string(),
                    description: format!("Temporary buffer with capacity {}", capacity),
                    version: "1.0.0".to_string(),
                    tags: vec![
                        "buffer".to_string(),
                        "storage".to_string(),
                        "category:DataSource".to_string(),
                    ],
                },
                capacity,
                data: Vec::with_capacity(capacity),
            }
        }

        /// Push an item into the buffer
        fn push(&mut self, item: String) -> Result<(), String> {
            if self.is_full() {
                return Err("Buffer is full".to_string());
            }
            self.data.push(item);
            Ok(())
        }

        /// Pop an item from the buffer
        fn pop(&mut self) -> Option<String> {
            self.data.pop()
        }

        /// Check if buffer is full
        fn is_full(&self) -> bool {
            self.data.len() >= self.capacity
        }

        /// Check if buffer is empty
        fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        /// Get current size
        fn size(&self) -> usize {
            self.data.len()
        }

        /// Clear all data
        fn clear(&mut self) {
            self.data.clear();
        }
    }

    impl Block for BufferBlock {
        fn id(&self) -> BlockId {
            self.id
        }

        fn metadata(&self) -> &BlockMetadata {
            &self.metadata
        }

        fn validate(&self) -> Result<(), String> {
            if self.capacity == 0 {
                return Err("Buffer capacity must be greater than 0".to_string());
            }
            if self.data.len() > self.capacity {
                return Err("Buffer data exceeds capacity".to_string());
            }
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn Block> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_buffer_block() {
        let mut buffer = BufferBlock::new(3);

        // Initially empty
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.size(), 0);

        // Push items
        assert!(buffer.push("item1".to_string()).is_ok());
        assert!(buffer.push("item2".to_string()).is_ok());
        assert!(buffer.push("item3".to_string()).is_ok());

        // Now full
        assert!(buffer.is_full());
        assert_eq!(buffer.size(), 3);

        // Can't push when full
        assert!(buffer.push("item4".to_string()).is_err());

        // Pop items
        assert_eq!(buffer.pop(), Some("item3".to_string()));
        assert_eq!(buffer.pop(), Some("item2".to_string()));
        assert_eq!(buffer.size(), 1);

        // Clear buffer
        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_buffer_validation() {
        let buffer = BufferBlock::new(10);
        assert!(buffer.validate().is_ok());

        let invalid_buffer = BufferBlock::new(0);
        assert!(invalid_buffer.validate().is_err());
    }

    // ============================================================================
    // Example 4: Using blocks with BlockRuntime
    // ============================================================================

    #[test]
    fn test_runtime_with_example_blocks() {
        let mut runtime = BlockRuntime::new();

        // Create and register different types of blocks
        let counter = Box::new(CounterBlock::new());
        let filter = Box::new(FilterBlock::new(50, false));
        let buffer = Box::new(BufferBlock::new(100));

        let counter_id = counter.id();
        let filter_id = filter.id();
        let buffer_id = buffer.id();

        // Register all blocks
        runtime.register_block(counter).unwrap();
        runtime.register_block(filter).unwrap();
        runtime.register_block(buffer).unwrap();

        // Verify all blocks are registered
        assert_eq!(runtime.block_count(), 3);

        // Retrieve and verify each block
        let retrieved_counter = runtime.get_block(counter_id);
        assert!(retrieved_counter.is_some());
        assert_eq!(retrieved_counter.unwrap().metadata().name, "Counter");

        let retrieved_filter = runtime.get_block(filter_id);
        assert!(retrieved_filter.is_some());
        assert_eq!(retrieved_filter.unwrap().metadata().name, "Filter");

        let retrieved_buffer = runtime.get_block(buffer_id);
        assert!(retrieved_buffer.is_some());
        assert_eq!(retrieved_buffer.unwrap().metadata().name, "Buffer");
    }

    // ============================================================================
    // Example 5: Block with rich metadata
    // ============================================================================

    /// A data source block with extensive metadata
    ///
    /// This demonstrates best practices for documenting blocks
    #[derive(Clone)]
    struct DataSourceBlock {
        id: BlockId,
        metadata: BlockMetadata,
        source_type: String,
    }

    impl DataSourceBlock {
        fn new(source_type: &str) -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: format!("{} Data Source", source_type),
                    description: format!(
                        "Reads data from {} sources and streams it to connected blocks",
                        source_type
                    ),
                    version: "2.1.0".to_string(),
                    tags: vec![
                        "source".to_string(),
                        "input".to_string(),
                        "data".to_string(),
                        format!("type:{}", source_type.to_lowercase()),
                        "category:DataSource".to_string(),
                    ],
                },
                source_type: source_type.to_string(),
            }
        }
    }

    impl Block for DataSourceBlock {
        fn id(&self) -> BlockId {
            self.id
        }

        fn metadata(&self) -> &BlockMetadata {
            &self.metadata
        }

        fn validate(&self) -> Result<(), String> {
            if self.source_type.is_empty() {
                return Err("Source type cannot be empty".to_string());
            }
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn Block> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_data_source_block_metadata() {
        let csv_source = DataSourceBlock::new("CSV");
        let json_source = DataSourceBlock::new("JSON");
        let db_source = DataSourceBlock::new("Database");

        // Verify metadata is properly formatted
        assert_eq!(csv_source.metadata().name, "CSV Data Source");
        assert_eq!(json_source.metadata().name, "JSON Data Source");
        assert_eq!(db_source.metadata().name, "Database Data Source");

        // Verify tags are correct
        assert!(csv_source.metadata().tags.contains(&"type:csv".to_string()));
        assert!(json_source.metadata().tags.contains(&"type:json".to_string()));
        assert!(db_source.metadata().tags.contains(&"type:database".to_string()));

        // All should have the data source category
        assert!(csv_source
            .metadata()
            .tags
            .contains(&"category:DataSource".to_string()));
    }

    // ============================================================================
    // Summary Test: Complete workflow
    // ============================================================================

    /// This test demonstrates a complete workflow:
    /// 1. Create multiple blocks with different purposes
    /// 2. Register them with a runtime
    /// 3. Verify they can be retrieved and used
    /// 4. Show how blocks maintain their state
    #[test]
    fn test_complete_workflow() {
        // Step 1: Create a runtime
        let mut runtime = BlockRuntime::new();

        // Step 2: Create various blocks
        let mut counter = CounterBlock::new();
        counter.increment();
        counter.increment();
        let counter_count = counter.get_count(); // Should be 2

        let filter = FilterBlock::new(100, false);
        let mut buffer = BufferBlock::new(5);
        buffer.push("data1".to_string()).unwrap();
        buffer.push("data2".to_string()).unwrap();

        let counter_id = counter.id();
        let filter_id = filter.id();
        let buffer_id = buffer.id();

        // Step 3: Register blocks
        runtime.register_block(Box::new(counter)).unwrap();
        runtime.register_block(Box::new(filter)).unwrap();
        runtime.register_block(Box::new(buffer)).unwrap();

        // Step 4: Verify runtime state
        assert_eq!(runtime.block_count(), 3);

        // Step 5: Retrieve and verify blocks
        assert!(runtime.get_block(counter_id).is_some());
        assert!(runtime.get_block(filter_id).is_some());
        assert!(runtime.get_block(buffer_id).is_some());

        // Step 6: Verify metadata
        let retrieved_counter = runtime.get_block(counter_id).unwrap();
        assert_eq!(retrieved_counter.metadata().name, "Counter");
        assert!(retrieved_counter.validate().is_ok());

        println!("✓ Successfully created and managed 3 different block types");
        println!("✓ Counter had {} operations", counter_count);
        println!("✓ Filter configured with threshold 100");
        println!("✓ Buffer contains 2 items");
    }
}
