//! Comprehensive tests for the block system
//!
//! This test module provides extensive testing and examples to help understand
//! how the block system works. Each test is well-documented to explain the concepts.

#[cfg(test)]
mod core_tests {
    use crate::core::{Block, BlockId, BlockMetadata};
    use crate::categories::BlockCategory;

    /// Test BlockId generation and uniqueness
    ///
    /// BlockId is a unique identifier for each block instance.
    /// Each new BlockId is randomly generated using UUID v4.
    #[test]
    fn test_block_id_creation_and_uniqueness() {
        // Create multiple block IDs
        let id1 = BlockId::new();
        let id2 = BlockId::new();
        let id3 = BlockId::new();

        // Each ID should be unique
        assert_ne!(id1, id2, "Block IDs should be unique");
        assert_ne!(id2, id3, "Block IDs should be unique");
        assert_ne!(id1, id3, "Block IDs should be unique");
    }

    /// Test BlockId serialization
    ///
    /// BlockIds can be serialized/deserialized, which is important for
    /// saving and loading block configurations.
    #[test]
    fn test_block_id_serialization() {
        let id = BlockId::new();

        // Serialize to JSON
        let json = serde_json::to_string(&id).expect("Failed to serialize BlockId");

        // Deserialize back
        let deserialized: BlockId = serde_json::from_str(&json)
            .expect("Failed to deserialize BlockId");

        // Should be identical
        assert_eq!(id, deserialized);
    }

    /// Test BlockMetadata creation
    ///
    /// BlockMetadata contains information about a block: its ID, name,
    /// description, version, and tags for categorization.
    #[test]
    fn test_block_metadata_creation() {
        let id = BlockId::new();
        let metadata = BlockMetadata {
            id,
            name: "Test Block".to_string(),
            description: "A block for testing".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["test".to_string(), "example".to_string()],
        };

        assert_eq!(metadata.id, id);
        assert_eq!(metadata.name, "Test Block");
        assert_eq!(metadata.description, "A block for testing");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.tags.len(), 2);
    }

    /// Test BlockMetadata serialization
    ///
    /// Metadata must be serializable for saving block configurations
    #[test]
    fn test_block_metadata_serialization() {
        let metadata = BlockMetadata {
            id: BlockId::new(),
            name: "Storage Block".to_string(),
            description: "Provides data storage".to_string(),
            version: "2.1.0".to_string(),
            tags: vec!["storage".to_string(), "category:DataSource".to_string()],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&metadata)
            .expect("Failed to serialize metadata");

        // Deserialize back
        let deserialized: BlockMetadata = serde_json::from_str(&json)
            .expect("Failed to deserialize metadata");

        assert_eq!(metadata.name, deserialized.name);
        assert_eq!(metadata.description, deserialized.description);
        assert_eq!(metadata.version, deserialized.version);
        assert_eq!(metadata.tags, deserialized.tags);
    }

    /// Example: Creating a simple block implementation
    ///
    /// This demonstrates how to implement the Block trait.
    /// A block must provide an ID, metadata, validation, and be clonable.
    #[derive(Clone)]
    struct SimpleBlock {
        id: BlockId,
        metadata: BlockMetadata,
    }

    impl SimpleBlock {
        fn new(name: &str, description: &str) -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: name.to_string(),
                    description: description.to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec![],
                },
            }
        }
    }

    impl Block for SimpleBlock {
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
    fn test_simple_block_implementation() {
        let block = SimpleBlock::new("Counter", "Counts operations");

        // Test Block trait methods
        let id = block.id();
        assert_eq!(id, block.metadata.id);

        let meta = block.metadata();
        assert_eq!(meta.name, "Counter");
        assert_eq!(meta.description, "Counts operations");

        // Test validation
        assert!(block.validate().is_ok());

        // Test cloning
        let cloned = block.clone_box();
        assert_eq!(cloned.id(), block.id());
    }

    #[test]
    fn test_block_validation_failure() {
        let mut block = SimpleBlock::new("Valid", "Description");
        block.metadata.name = "".to_string(); // Make it invalid

        let result = block.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Block name cannot be empty");
    }
}

#[cfg(test)]
mod category_tests {
    use crate::categories::BlockCategory;

    /// Test BlockCategory display names
    ///
    /// Categories help organize blocks by their function.
    #[test]
    fn test_category_display_names() {
        assert_eq!(BlockCategory::DataSource.display_name(), "Data Source");
        assert_eq!(BlockCategory::Transformation.display_name(), "Transformation");
        assert_eq!(BlockCategory::Aggregation.display_name(), "Aggregation");
        assert_eq!(BlockCategory::Output.display_name(), "Output");
        assert_eq!(BlockCategory::ControlFlow.display_name(), "Control Flow");
    }

    /// Test custom category
    ///
    /// You can create custom categories for domain-specific blocks
    #[test]
    fn test_custom_category() {
        let category = BlockCategory::Custom("Machine Learning".to_string());
        assert_eq!(category.display_name(), "Machine Learning");
    }

    /// Test category serialization
    ///
    /// Categories must be serializable for saving configurations
    #[test]
    fn test_category_serialization() {
        let category = BlockCategory::Transformation;

        let json = serde_json::to_string(&category).unwrap();
        let deserialized: BlockCategory = serde_json::from_str(&json).unwrap();

        assert_eq!(category, deserialized);
    }

    /// Test custom category serialization
    #[test]
    fn test_custom_category_serialization() {
        let category = BlockCategory::Custom("Neural Network".to_string());

        let json = serde_json::to_string(&category).unwrap();
        let deserialized: BlockCategory = serde_json::from_str(&json).unwrap();

        assert_eq!(category, deserialized);
    }

    /// Test category equality
    #[test]
    fn test_category_equality() {
        assert_eq!(BlockCategory::DataSource, BlockCategory::DataSource);
        assert_ne!(BlockCategory::DataSource, BlockCategory::Output);

        let custom1 = BlockCategory::Custom("ML".to_string());
        let custom2 = BlockCategory::Custom("ML".to_string());
        assert_eq!(custom1, custom2);
    }
}

#[cfg(test)]
mod runtime_tests {
    use crate::core::{Block, BlockId, BlockMetadata};
    use crate::runtime::BlockRuntime;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestBlock {
        id: BlockId,
        metadata: BlockMetadata,
    }

    impl TestBlock {
        fn new(name: &str) -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: name.to_string(),
                    description: "Test block".to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec![],
                },
            }
        }
    }

    impl Block for TestBlock {
        fn id(&self) -> BlockId {
            self.id
        }

        fn metadata(&self) -> &BlockMetadata {
            &self.metadata
        }

        fn validate(&self) -> Result<(), String> {
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn Block> {
            Box::new(self.clone())
        }
    }

    /// Test BlockRuntime creation
    ///
    /// BlockRuntime manages the execution of blocks
    #[test]
    fn test_runtime_creation() {
        let runtime = BlockRuntime::new();
        assert_eq!(runtime.block_count(), 0);
    }

    /// Test block registration in runtime
    ///
    /// Blocks must be registered with the runtime before they can be executed
    #[test]
    fn test_block_registration_in_runtime() {
        let mut runtime = BlockRuntime::new();
        let block = Box::new(TestBlock::new("TestBlock"));
        let block_id = block.id();

        let result = runtime.register_block(block);
        assert!(result.is_ok());
        assert_eq!(runtime.block_count(), 1);

        // Verify we can retrieve the block
        let retrieved = runtime.get_block(block_id);
        assert!(retrieved.is_some());
    }

    /// Test retrieving registered blocks
    #[test]
    fn test_get_registered_block() {
        let mut runtime = BlockRuntime::new();
        let block = Box::new(TestBlock::new("MyBlock"));
        let block_id = block.id();

        runtime.register_block(block).unwrap();

        let retrieved = runtime.get_block(block_id);
        assert!(retrieved.is_some());

        let retrieved_block = retrieved.unwrap();
        assert_eq!(retrieved_block.metadata().name, "MyBlock");
    }

    /// Test getting non-existent block
    #[test]
    fn test_get_nonexistent_block() {
        let runtime = BlockRuntime::new();
        let fake_id = BlockId::new();

        let result = runtime.get_block(fake_id);
        assert!(result.is_none());
    }

    /// Test multiple block registrations
    #[test]
    fn test_multiple_block_registrations() {
        let mut runtime = BlockRuntime::new();

        let block1 = Box::new(TestBlock::new("Block1"));
        let block2 = Box::new(TestBlock::new("Block2"));
        let block3 = Box::new(TestBlock::new("Block3"));

        runtime.register_block(block1).unwrap();
        runtime.register_block(block2).unwrap();
        runtime.register_block(block3).unwrap();

        assert_eq!(runtime.block_count(), 3);
    }
}
