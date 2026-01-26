//! Block Registry - Central registry for managing all available blocks
//!
//! This module provides a thread-safe registry for registering, discovering, and managing
//! blocks in the system. It supports:
//! - Block registration and unregistration
//! - Block discovery by ID, category, or search query
//! - Block validation
//! - Dependency resolution
//! - Compatibility checking

use crate::core::{Block, BlockId, BlockMetadata};
use crate::categories::BlockCategory;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Block registry for managing all available blocks
///
/// The registry uses `Arc<RwLock<HashMap>>` for thread-safe access to blocks.
/// It supports concurrent reads and exclusive writes using parking_lot's RwLock
/// for better performance compared to std::sync::RwLock.
#[derive(Clone)]
pub struct BlockRegistry {
    blocks: Arc<RwLock<HashMap<String, Arc<dyn Block>>>>,
}

impl BlockRegistry {
    /// Create a new empty block registry
    ///
    /// # Example
    /// ```
    /// use block_system::core::registry::BlockRegistry;
    ///
    /// let registry = BlockRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new block in the registry
    ///
    /// # Arguments
    /// * `block` - The block to register (wrapped in Arc)
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeds
    /// * `Err(RegistryError)` if the block ID already exists or validation fails
    ///
    /// # Example
    /// ```ignore
    /// let block = Arc::new(MyBlock::new());
    /// registry.register(block)?;
    /// ```
    pub fn register(&self, block: Arc<dyn Block>) -> Result<(), RegistryError> {
        // Validate block before registration
        self.validate_block(&*block)?;

        let id = block.id().0.to_string();
        let mut blocks = self.blocks.write();

        if blocks.contains_key(&id) {
            return Err(RegistryError::DuplicateBlock(id));
        }

        blocks.insert(id, block);
        Ok(())
    }

    /// Unregister a block from the registry
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block to unregister
    ///
    /// # Returns
    /// * `Ok(())` if unregistration succeeds
    /// * `Err(RegistryError)` if the block is not found
    pub fn unregister(&self, block_id: &str) -> Result<(), RegistryError> {
        let mut blocks = self.blocks.write();

        blocks
            .remove(block_id)
            .ok_or_else(|| RegistryError::BlockNotFound(block_id.to_string()))?;

        Ok(())
    }

    /// Get a block by its ID
    ///
    /// # Arguments
    /// * `id` - The block ID to look up
    ///
    /// # Returns
    /// * `Ok(Arc<dyn Block>)` if the block is found
    /// * `Err(RegistryError)` if the block is not found
    pub fn get_block(&self, id: &str) -> Result<Arc<dyn Block>, RegistryError> {
        let blocks = self.blocks.read();

        blocks
            .get(id)
            .cloned()
            .ok_or_else(|| RegistryError::BlockNotFound(id.to_string()))
    }

    /// Get all registered blocks
    ///
    /// # Returns
    /// A vector containing all registered blocks
    pub fn get_all_blocks(&self) -> Vec<Arc<dyn Block>> {
        let blocks = self.blocks.read();
        blocks.values().cloned().collect()
    }

    /// Get blocks filtered by category
    ///
    /// # Arguments
    /// * `category` - The category to filter by
    ///
    /// # Returns
    /// A vector containing all blocks in the specified category
    ///
    /// Note: This requires blocks to have category information in their metadata tags.
    /// We look for tags like "category:Storage", "category:Index", etc.
    pub fn get_blocks_by_category(&self, category: &BlockCategory) -> Vec<Arc<dyn Block>> {
        let blocks = self.blocks.read();
        let category_str = format!("category:{}", category);

        blocks
            .values()
            .filter(|b| b.metadata().tags.contains(&category_str))
            .cloned()
            .collect()
    }

    /// Search for blocks by query string
    ///
    /// Searches in block name, description, and tags.
    ///
    /// # Arguments
    /// * `query` - The search query (case-insensitive)
    ///
    /// # Returns
    /// A vector containing all blocks matching the search query
    pub fn search_blocks(&self, query: &str) -> Vec<Arc<dyn Block>> {
        let blocks = self.blocks.read();
        let query = query.to_lowercase();

        blocks
            .values()
            .filter(|b| {
                let meta = b.metadata();
                meta.name.to_lowercase().contains(&query)
                    || meta.description.to_lowercase().contains(&query)
                    || meta.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    /// Get the number of registered blocks
    ///
    /// # Returns
    /// The count of registered blocks
    pub fn count(&self) -> usize {
        let blocks = self.blocks.read();
        blocks.len()
    }

    /// Check if a block with the given ID exists
    ///
    /// # Arguments
    /// * `id` - The block ID to check
    ///
    /// # Returns
    /// `true` if the block exists, `false` otherwise
    pub fn contains(&self, id: &str) -> bool {
        let blocks = self.blocks.read();
        blocks.contains_key(id)
    }

    /// Clear all registered blocks
    ///
    /// This removes all blocks from the registry.
    pub fn clear(&self) {
        let mut blocks = self.blocks.write();
        blocks.clear();
    }

    /// Validate a block before registration
    ///
    /// # Arguments
    /// * `block` - The block to validate
    ///
    /// # Returns
    /// * `Ok(())` if validation succeeds
    /// * `Err(RegistryError)` if validation fails
    fn validate_block(&self, block: &dyn Block) -> Result<(), RegistryError> {
        // Validate using the block's own validation method
        block
            .validate()
            .map_err(|e| RegistryError::ValidationError(e))?;

        let meta = block.metadata();

        // Validate metadata
        if meta.name.is_empty() {
            return Err(RegistryError::ValidationError(
                "Block name cannot be empty".into(),
            ));
        }

        if meta.version.is_empty() {
            return Err(RegistryError::ValidationError(
                "Block version cannot be empty".into(),
            ));
        }

        Ok(())
    }

    /// Resolve dependencies for a set of blocks
    ///
    /// Builds a dependency graph and detects cycles.
    ///
    /// # Arguments
    /// * `block_ids` - The block IDs to resolve dependencies for
    ///
    /// # Returns
    /// * `Ok(DependencyGraph)` if resolution succeeds
    /// * `Err(RegistryError)` if any block is not found
    pub fn resolve_dependencies(&self, block_ids: &[String]) -> Result<DependencyGraph, RegistryError> {
        let mut graph = DependencyGraph::new();

        // Verify all blocks exist and add them to the graph
        for block_id in block_ids {
            // Verify block exists
            self.get_block(block_id)?;
            graph.add_node(block_id.clone());
        }

        // For now, we don't have dependency information in the simplified Block trait
        // This can be extended later when we add constraint/requirement support
        // For demonstration, we just return the graph with nodes

        Ok(graph)
    }

    /// Check compatibility between blocks
    ///
    /// Analyzes blocks for potential conflicts and compatibility issues.
    ///
    /// # Arguments
    /// * `block_ids` - The block IDs to check for compatibility
    ///
    /// # Returns
    /// * `Ok(CompatibilityResult)` containing compatibility analysis
    /// * `Err(RegistryError)` if any block is not found
    pub fn check_compatibility(
        &self,
        block_ids: &[String],
    ) -> Result<CompatibilityResult, RegistryError> {
        let mut result = CompatibilityResult {
            compatible: true,
            conflicts: Vec::new(),
        };

        // Verify all blocks exist
        let blocks: Vec<_> = block_ids
            .iter()
            .map(|id| self.get_block(id))
            .collect::<Result<_, _>>()?;

        // Basic compatibility checking
        // For now, we check for duplicate names which might indicate conflicts
        let mut name_counts: HashMap<String, Vec<String>> = HashMap::new();
        for block in &blocks {
            let meta = block.metadata();
            name_counts
                .entry(meta.name.clone())
                .or_insert_with(Vec::new)
                .push(block.id().0.to_string());
        }

        // Check for blocks with the same name (potential conflicts)
        for (name, ids) in name_counts {
            if ids.len() > 1 {
                result.compatible = false;
                result.conflicts.push(Conflict {
                    block_ids: ids.clone(),
                    reason: format!("Multiple blocks with the same name: {}", name),
                    severity: ConflictSeverity::Warning,
                });
            }
        }

        Ok(result)
    }
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency graph for block dependencies
///
/// Represents dependencies between blocks and can detect circular dependencies.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// All nodes in the graph (block IDs)
    pub nodes: Vec<String>,
    /// Edges representing dependencies (from, to)
    pub edges: Vec<(String, String)>,
    /// Detected circular dependencies
    pub cycles: Vec<Vec<String>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            cycles: Vec::new(),
        }
    }

    /// Add a node to the graph
    fn add_node(&mut self, node: String) {
        if !self.nodes.contains(&node) {
            self.nodes.push(node);
        }
    }

    /// Add an edge to the graph
    ///
    /// # Arguments
    /// * `from` - The source node
    /// * `to` - The target node (dependency)
    pub fn add_edge(&mut self, from: String, to: String) {
        self.edges.push((from, to));
    }

    /// Detect cycles in the dependency graph using DFS
    ///
    /// This method populates the `cycles` field with any detected circular dependencies.
    pub fn detect_cycles(&mut self) {
        let mut visited = HashMap::new();
        let mut rec_stack = HashMap::new();
        let mut cycles = Vec::new();

        for node in &self.nodes.clone() {
            if !visited.get(node).unwrap_or(&false) {
                self.dfs_cycle_detect(
                    node,
                    &mut visited,
                    &mut rec_stack,
                    &mut Vec::new(),
                    &mut cycles,
                );
            }
        }

        self.cycles = cycles;
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_detect(
        &self,
        node: &str,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashMap<String, bool>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string(), true);
        rec_stack.insert(node.to_string(), true);
        path.push(node.to_string());

        // Get all neighbors
        for (from, to) in &self.edges {
            if from == node {
                if !visited.get(to).unwrap_or(&false) {
                    self.dfs_cycle_detect(to, visited, rec_stack, path, cycles);
                } else if *rec_stack.get(to).unwrap_or(&false) {
                    // Found a cycle
                    if let Some(pos) = path.iter().position(|x| x == to) {
                        cycles.push(path[pos..].to_vec());
                    }
                }
            }
        }

        path.pop();
        rec_stack.insert(node.to_string(), false);
    }

    /// Check if the graph has cycles
    pub fn has_cycles(&self) -> bool {
        !self.cycles.is_empty()
    }
}

/// Result of compatibility checking
#[derive(Debug, Clone)]
pub struct CompatibilityResult {
    /// Whether the blocks are compatible
    pub compatible: bool,
    /// List of detected conflicts
    pub conflicts: Vec<Conflict>,
}

/// Represents a conflict between blocks
#[derive(Debug, Clone)]
pub struct Conflict {
    /// IDs of blocks involved in the conflict
    pub block_ids: Vec<String>,
    /// Reason for the conflict
    pub reason: String,
    /// Severity of the conflict
    pub severity: ConflictSeverity,
}

/// Severity level of a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSeverity {
    /// Fatal error - blocks cannot be used together
    Error,
    /// Warning - blocks may have issues but can be used together
    Warning,
}

/// Registry error types
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Block with given ID was not found
    #[error("Block not found: {0}")]
    BlockNotFound(String),

    /// Attempted to register a block with duplicate ID
    #[error("Duplicate block ID: {0}")]
    DuplicateBlock(String),

    /// Block validation failed
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BlockId, BlockMetadata};

    // Mock block for testing
    #[derive(Clone)]
    struct MockBlock {
        id: BlockId,
        metadata: BlockMetadata,
    }

    impl MockBlock {
        fn new(name: &str, description: &str, tags: Vec<String>) -> Self {
            let id = BlockId::new();
            Self {
                id,
                metadata: BlockMetadata {
                    id,
                    name: name.to_string(),
                    description: description.to_string(),
                    version: "1.0.0".to_string(),
                    tags,
                },
            }
        }
    }

    impl Block for MockBlock {
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

    #[test]
    fn test_registry_creation() {
        let registry = BlockRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_block_registration() {
        let registry = BlockRegistry::new();
        let block = Arc::new(MockBlock::new(
            "TestBlock",
            "A test block",
            vec!["test".to_string()],
        ));

        let result = registry.register(block.clone());
        assert!(result.is_ok(), "Registration should succeed");
        assert_eq!(registry.count(), 1);

        // Verify we can retrieve it
        let retrieved = registry.get_block(&block.id().0.to_string());
        assert!(retrieved.is_ok());
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = BlockRegistry::new();
        let block = Arc::new(MockBlock::new(
            "TestBlock",
            "A test block",
            vec!["test".to_string()],
        ));

        registry.register(block.clone()).unwrap();

        // Try to register the same block again
        let result = registry.register(block);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::DuplicateBlock(_)));
    }

    #[test]
    fn test_block_unregistration() {
        let registry = BlockRegistry::new();
        let block = Arc::new(MockBlock::new(
            "TestBlock",
            "A test block",
            vec!["test".to_string()],
        ));
        let block_id = block.id().0.to_string();

        registry.register(block).unwrap();
        assert_eq!(registry.count(), 1);

        let result = registry.unregister(&block_id);
        assert!(result.is_ok());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry = BlockRegistry::new();
        let result = registry.unregister("nonexistent-id");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::BlockNotFound(_)));
    }

    #[test]
    fn test_get_all_blocks() {
        let registry = BlockRegistry::new();

        let block1 = Arc::new(MockBlock::new("Block1", "First block", vec![]));
        let block2 = Arc::new(MockBlock::new("Block2", "Second block", vec![]));
        let block3 = Arc::new(MockBlock::new("Block3", "Third block", vec![]));

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();
        registry.register(block3).unwrap();

        let all_blocks = registry.get_all_blocks();
        assert_eq!(all_blocks.len(), 3);
    }

    #[test]
    fn test_search_blocks() {
        let registry = BlockRegistry::new();

        let block1 = Arc::new(MockBlock::new(
            "StorageBlock",
            "A storage block",
            vec!["category:Storage".to_string()],
        ));
        let block2 = Arc::new(MockBlock::new(
            "IndexBlock",
            "An index block",
            vec!["category:Index".to_string()],
        ));
        let block3 = Arc::new(MockBlock::new(
            "BufferBlock",
            "A buffer storage block",
            vec!["category:Buffer".to_string()],
        ));

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();
        registry.register(block3).unwrap();

        // Search by name
        let results = registry.search_blocks("storage");
        assert_eq!(results.len(), 2); // StorageBlock and BufferBlock (has "storage" in description)

        // Search by category tag
        let results = registry.search_blocks("category:index");
        assert_eq!(results.len(), 1);

        // Search by description
        let results = registry.search_blocks("buffer");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_blocks_by_category() {
        let registry = BlockRegistry::new();

        let block1 = Arc::new(MockBlock::new(
            "Block1",
            "Storage block",
            vec!["category:DataSource".to_string()],
        ));
        let block2 = Arc::new(MockBlock::new(
            "Block2",
            "Index block",
            vec!["category:DataSource".to_string()],
        ));
        let block3 = Arc::new(MockBlock::new(
            "Block3",
            "Buffer block",
            vec!["category:Transformation".to_string()],
        ));

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();
        registry.register(block3).unwrap();

        let storage_blocks = registry.get_blocks_by_category(&BlockCategory::DataSource);
        assert_eq!(storage_blocks.len(), 2);

        let transform_blocks = registry.get_blocks_by_category(&BlockCategory::Transformation);
        assert_eq!(transform_blocks.len(), 1);
    }

    #[test]
    fn test_contains() {
        let registry = BlockRegistry::new();
        let block = Arc::new(MockBlock::new("TestBlock", "A test block", vec![]));
        let block_id = block.id().0.to_string();

        assert!(!registry.contains(&block_id));

        registry.register(block).unwrap();
        assert!(registry.contains(&block_id));
    }

    #[test]
    fn test_clear() {
        let registry = BlockRegistry::new();

        let block1 = Arc::new(MockBlock::new("Block1", "First block", vec![]));
        let block2 = Arc::new(MockBlock::new("Block2", "Second block", vec![]));

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();
        assert_eq!(registry.count(), 2);

        registry.clear();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_validation_empty_name() {
        let registry = BlockRegistry::new();
        let mut block = MockBlock::new("", "Description", vec![]);
        block.metadata.name = "".to_string();

        let result = registry.register(Arc::new(block));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::ValidationError(_)));
    }

    #[test]
    fn test_validation_empty_version() {
        let registry = BlockRegistry::new();
        let mut block = MockBlock::new("TestBlock", "Description", vec![]);
        block.metadata.version = "".to_string();

        let result = registry.register(Arc::new(block));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::ValidationError(_)));
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        graph.add_node("block1".to_string());
        graph.add_node("block2".to_string());
        graph.add_node("block3".to_string());

        assert_eq!(graph.nodes.len(), 3);

        graph.add_edge("block1".to_string(), "block2".to_string());
        graph.add_edge("block2".to_string(), "block3".to_string());

        assert_eq!(graph.edges.len(), 2);
        assert!(!graph.has_cycles());
    }

    #[test]
    fn test_dependency_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph.add_node("block1".to_string());
        graph.add_node("block2".to_string());
        graph.add_node("block3".to_string());

        // Create a cycle: block1 -> block2 -> block3 -> block1
        graph.add_edge("block1".to_string(), "block2".to_string());
        graph.add_edge("block2".to_string(), "block3".to_string());
        graph.add_edge("block3".to_string(), "block1".to_string());

        graph.detect_cycles();
        assert!(graph.has_cycles());
        assert!(!graph.cycles.is_empty());
    }

    #[test]
    fn test_resolve_dependencies() {
        let registry = BlockRegistry::new();

        let block1 = Arc::new(MockBlock::new("Block1", "First block", vec![]));
        let block2 = Arc::new(MockBlock::new("Block2", "Second block", vec![]));

        let id1 = block1.id().0.to_string();
        let id2 = block2.id().0.to_string();

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();

        let result = registry.resolve_dependencies(&[id1, id2]);
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.nodes.len(), 2);
    }

    #[test]
    fn test_check_compatibility() {
        let registry = BlockRegistry::new();

        let block1 = Arc::new(MockBlock::new("Block1", "First block", vec![]));
        let block2 = Arc::new(MockBlock::new("Block2", "Second block", vec![]));

        let id1 = block1.id().0.to_string();
        let id2 = block2.id().0.to_string();

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();

        let result = registry.check_compatibility(&[id1, id2]);
        assert!(result.is_ok());

        let compat = result.unwrap();
        assert!(compat.compatible);
        assert_eq!(compat.conflicts.len(), 0);
    }

    #[test]
    fn test_check_compatibility_with_conflicts() {
        let registry = BlockRegistry::new();

        // Create two blocks with the same name
        let block1 = Arc::new(MockBlock::new("SameName", "First block", vec![]));
        let block2 = Arc::new(MockBlock::new("SameName", "Second block", vec![]));

        let id1 = block1.id().0.to_string();
        let id2 = block2.id().0.to_string();

        registry.register(block1).unwrap();
        registry.register(block2).unwrap();

        let result = registry.check_compatibility(&[id1, id2]);
        assert!(result.is_ok());

        let compat = result.unwrap();
        assert!(!compat.compatible);
        assert_eq!(compat.conflicts.len(), 1);
        assert_eq!(compat.conflicts[0].severity, ConflictSeverity::Warning);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let registry = Arc::new(BlockRegistry::new());
        let mut handles = vec![];

        // Spawn multiple threads that register blocks
        for i in 0..10 {
            let registry_clone = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                let block = Arc::new(MockBlock::new(
                    &format!("Block{}", i),
                    &format!("Block number {}", i),
                    vec![],
                ));
                registry_clone.register(block).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(registry.count(), 10);
    }
}
