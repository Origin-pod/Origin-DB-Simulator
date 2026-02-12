//! Graph validation engine
//!
//! Validates a block graph before execution: cycle detection, port connectivity,
//! type compatibility, and parameter constraint checking. Produces a
//! `GraphValidationResult` that mirrors the frontend's `ValidationResult`.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::block::Block;
use crate::core::port::{Connection, Port, PortDirection, PortType};

// ── Result types ────────────────────────────────────────────────────────────

/// A single validation error with optional location and suggestion.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Block id where the problem was found (if applicable).
    pub node_id: Option<String>,
    /// Human-readable description.
    pub message: String,
    /// Optional suggestion for how to fix it.
    pub suggestion: Option<String>,
}

/// A non-fatal warning.
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub node_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Overall validation result — mirrors frontend `ValidationResult`.
#[derive(Debug, Clone)]
pub struct GraphValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl GraphValidationResult {
    fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn add_error(&mut self, node_id: Option<&str>, message: impl Into<String>, suggestion: Option<&str>) {
        self.valid = false;
        self.errors.push(ValidationError {
            node_id: node_id.map(|s| s.to_string()),
            message: message.into(),
            suggestion: suggestion.map(|s| s.to_string()),
        });
    }

    fn add_warning(&mut self, node_id: Option<&str>, message: impl Into<String>, suggestion: Option<&str>) {
        self.warnings.push(ValidationWarning {
            node_id: node_id.map(|s| s.to_string()),
            message: message.into(),
            suggestion: suggestion.map(|s| s.to_string()),
        });
    }

    /// Merge another result into this one.
    fn merge(&mut self, other: GraphValidationResult) {
        if !other.valid {
            self.valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

// ── Validator ───────────────────────────────────────────────────────────────

/// Validates a graph of blocks and connections.
pub struct GraphValidator;

impl GraphValidator {
    /// Run every validation check against the given blocks and connections.
    ///
    /// `entry_points` lists block IDs that receive data from outside the graph
    /// (e.g. from the workload generator). Their required input ports are not
    /// required to have incoming connections.
    pub fn validate(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
        entry_points: &[&str],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();

        result.merge(Self::check_referenced_blocks_exist(blocks, connections));
        result.merge(Self::check_duplicate_connections(connections));
        result.merge(Self::check_port_existence(blocks, connections));
        result.merge(Self::check_port_directions(blocks, connections));
        result.merge(Self::check_port_type_compatibility(blocks, connections));
        result.merge(Self::check_required_inputs_connected(blocks, connections, entry_points));
        result.merge(Self::check_multiple_connections(blocks, connections));
        result.merge(Self::check_cycles(blocks, connections));
        result.merge(Self::check_disconnected_blocks(blocks, connections));

        result
    }

    // ── Individual checks ───────────────────────────────────────────────

    /// Every block_id referenced in a connection must exist.
    fn check_referenced_blocks_exist(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();
        for conn in connections {
            if !blocks.contains_key(&conn.source_block_id) {
                result.add_error(
                    Some(&conn.source_block_id),
                    format!("Connection '{}' references unknown source block '{}'", conn.id, conn.source_block_id),
                    Some("Add the block to the graph or remove the connection"),
                );
            }
            if !blocks.contains_key(&conn.target_block_id) {
                result.add_error(
                    Some(&conn.target_block_id),
                    format!("Connection '{}' references unknown target block '{}'", conn.id, conn.target_block_id),
                    Some("Add the block to the graph or remove the connection"),
                );
            }
        }
        result
    }

    /// No two connections should have the same (source_block, source_port, target_block, target_port).
    fn check_duplicate_connections(connections: &[Connection]) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();
        let mut seen = HashSet::new();
        for conn in connections {
            let key = (
                conn.source_block_id.as_str(),
                conn.source_port_id.as_str(),
                conn.target_block_id.as_str(),
                conn.target_port_id.as_str(),
            );
            if !seen.insert(key) {
                result.add_error(
                    None,
                    format!(
                        "Duplicate connection from {}:{} to {}:{}",
                        conn.source_block_id, conn.source_port_id,
                        conn.target_block_id, conn.target_port_id,
                    ),
                    Some("Remove the duplicate connection"),
                );
            }
        }
        result
    }

    /// Ports referenced by connections must exist on their blocks.
    fn check_port_existence(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();
        for conn in connections {
            if let Some(block) = blocks.get(&conn.source_block_id) {
                let all_ports: Vec<&Port> = block.outputs().iter().chain(block.inputs().iter()).collect();
                if !all_ports.iter().any(|p| p.id == conn.source_port_id) {
                    result.add_error(
                        Some(&conn.source_block_id),
                        format!(
                            "Block '{}' has no port '{}'",
                            conn.source_block_id, conn.source_port_id
                        ),
                        Some("Check port names match the block definition"),
                    );
                }
            }
            if let Some(block) = blocks.get(&conn.target_block_id) {
                let all_ports: Vec<&Port> = block.inputs().iter().chain(block.outputs().iter()).collect();
                if !all_ports.iter().any(|p| p.id == conn.target_port_id) {
                    result.add_error(
                        Some(&conn.target_block_id),
                        format!(
                            "Block '{}' has no port '{}'",
                            conn.target_block_id, conn.target_port_id
                        ),
                        Some("Check port names match the block definition"),
                    );
                }
            }
        }
        result
    }

    /// Source must be an output port and target must be an input port.
    fn check_port_directions(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();
        for conn in connections {
            if let Some(block) = blocks.get(&conn.source_block_id) {
                if let Some(port) = Self::find_port(block.as_ref(), &conn.source_port_id) {
                    if port.direction != PortDirection::Output {
                        result.add_error(
                            Some(&conn.source_block_id),
                            format!(
                                "Port '{}' on block '{}' is not an output port",
                                conn.source_port_id, conn.source_block_id
                            ),
                            Some("Source side of a connection must be an output port"),
                        );
                    }
                }
            }
            if let Some(block) = blocks.get(&conn.target_block_id) {
                if let Some(port) = Self::find_port(block.as_ref(), &conn.target_port_id) {
                    if port.direction != PortDirection::Input {
                        result.add_error(
                            Some(&conn.target_block_id),
                            format!(
                                "Port '{}' on block '{}' is not an input port",
                                conn.target_port_id, conn.target_block_id
                            ),
                            Some("Target side of a connection must be an input port"),
                        );
                    }
                }
            }
        }
        result
    }

    /// Connected ports must have compatible types.
    fn check_port_type_compatibility(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();
        for conn in connections {
            let src_port = blocks
                .get(&conn.source_block_id)
                .and_then(|b| Self::find_port(b.as_ref(), &conn.source_port_id));
            let tgt_port = blocks
                .get(&conn.target_block_id)
                .and_then(|b| Self::find_port(b.as_ref(), &conn.target_port_id));

            if let (Some(src), Some(tgt)) = (src_port, tgt_port) {
                if !Self::types_compatible(src.port_type, tgt.port_type) {
                    result.add_error(
                        None,
                        format!(
                            "Incompatible port types: {}:{} ({:?}) → {}:{} ({:?})",
                            conn.source_block_id, conn.source_port_id, src.port_type,
                            conn.target_block_id, conn.target_port_id, tgt.port_type,
                        ),
                        Some("Connected ports must have the same type, or Stream/Batch → Batch/Stream"),
                    );
                }
            }
        }
        result
    }

    /// Every required input port must have at least one incoming connection,
    /// unless the block is an entry point (fed from outside the graph).
    fn check_required_inputs_connected(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
        entry_points: &[&str],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();
        let entry_set: HashSet<&str> = entry_points.iter().copied().collect();

        // Build set of (target_block_id, target_port_id) that have connections.
        let connected_inputs: HashSet<(&str, &str)> = connections
            .iter()
            .map(|c| (c.target_block_id.as_str(), c.target_port_id.as_str()))
            .collect();

        for (block_id, block) in blocks {
            // Entry-point blocks get data from outside the graph.
            if entry_set.contains(block_id.as_str()) {
                continue;
            }
            for port in block.inputs() {
                if port.required && !connected_inputs.contains(&(block_id.as_str(), port.id.as_str())) {
                    result.add_error(
                        Some(block_id),
                        format!(
                            "Required input port '{}' on block '{}' is not connected",
                            port.id, block_id
                        ),
                        Some("Connect a source to this input port"),
                    );
                }
            }
        }
        result
    }

    /// If a port does not accept multiple connections, check it has at most one.
    fn check_multiple_connections(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();

        // Count connections per input port.
        let mut input_conn_count: HashMap<(&str, &str), usize> = HashMap::new();
        for conn in connections {
            *input_conn_count
                .entry((conn.target_block_id.as_str(), conn.target_port_id.as_str()))
                .or_default() += 1;
        }

        for ((block_id, port_id), count) in &input_conn_count {
            if *count > 1 {
                if let Some(block) = blocks.get(*block_id) {
                    if let Some(port) = Self::find_port(block.as_ref(), port_id) {
                        if !port.multiple {
                            result.add_error(
                                Some(block_id),
                                format!(
                                    "Input port '{}' on block '{}' has {} connections but does not accept multiple",
                                    port_id, block_id, count
                                ),
                                Some("Remove extra connections or enable multiple on the port"),
                            );
                        }
                    }
                }
            }
        }
        result
    }

    /// Cycle detection using Kahn's algorithm (topological sort).
    /// If we can't sort all nodes, the graph has a cycle.
    fn check_cycles(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();

        // Build adjacency list and in-degree map.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in blocks.keys() {
            in_degree.entry(id.as_str()).or_insert(0);
            adj.entry(id.as_str()).or_default();
        }

        for conn in connections {
            // Only count edges between blocks that exist.
            if blocks.contains_key(&conn.source_block_id)
                && blocks.contains_key(&conn.target_block_id)
            {
                adj.entry(conn.source_block_id.as_str())
                    .or_default()
                    .push(conn.target_block_id.as_str());
                *in_degree.entry(conn.target_block_id.as_str()).or_default() += 1;
            }
        }

        // Kahn's algorithm.
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut visited = 0usize;

        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if visited < blocks.len() {
            // Find blocks that are part of the cycle (in_degree > 0).
            let cycle_blocks: Vec<&str> = in_degree
                .iter()
                .filter(|(_, &deg)| deg > 0)
                .map(|(&id, _)| id)
                .collect();

            result.add_error(
                None,
                format!(
                    "Graph contains a cycle involving blocks: [{}]",
                    cycle_blocks.join(", ")
                ),
                Some("Remove connections to break the cycle"),
            );
        }

        result
    }

    /// Warn about blocks that have no connections at all.
    fn check_disconnected_blocks(
        blocks: &HashMap<String, Box<dyn Block>>,
        connections: &[Connection],
    ) -> GraphValidationResult {
        let mut result = GraphValidationResult::ok();

        let mut connected: HashSet<&str> = HashSet::new();
        for conn in connections {
            connected.insert(conn.source_block_id.as_str());
            connected.insert(conn.target_block_id.as_str());
        }

        for block_id in blocks.keys() {
            if !connected.contains(block_id.as_str()) && blocks.len() > 1 {
                result.add_warning(
                    Some(block_id),
                    format!("Block '{}' is not connected to any other block", block_id),
                    Some("Connect this block or remove it from the graph"),
                );
            }
        }
        result
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    /// Find a port definition by id across both inputs and outputs.
    fn find_port<'a>(block: &'a dyn Block, port_id: &str) -> Option<&'a Port> {
        block
            .inputs()
            .iter()
            .chain(block.outputs().iter())
            .find(|p| p.id == port_id)
    }

    /// Two port types are compatible if they're equal, or if one is
    /// Stream and the other is Batch (both are record collections).
    fn types_compatible(src: PortType, tgt: PortType) -> bool {
        if src == tgt {
            return true;
        }
        matches!(
            (src, tgt),
            (PortType::DataStream, PortType::Batch)
                | (PortType::Batch, PortType::DataStream)
        )
    }

    /// Compute topological order of block ids. Returns `None` if graph
    /// has a cycle.  Used by the engine for execution ordering.
    pub fn topological_sort(
        block_ids: &[&str],
        connections: &[Connection],
    ) -> Option<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for &id in block_ids {
            in_degree.entry(id).or_insert(0);
            adj.entry(id).or_default();
        }

        let id_set: HashSet<&str> = block_ids.iter().copied().collect();

        for conn in connections {
            let src = conn.source_block_id.as_str();
            let tgt = conn.target_block_id.as_str();
            if id_set.contains(src) && id_set.contains(tgt) {
                adj.entry(src).or_default().push(tgt);
                *in_degree.entry(tgt).or_default() += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();

        while let Some(node) = queue.pop_front() {
            order.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if order.len() == block_ids.len() {
            Some(order)
        } else {
            None
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::storage::HeapFileBlock;
    use crate::categories::index::BTreeIndexBlock;
    use crate::categories::buffer::LRUBufferBlock;
    use crate::core::port::Connection;

    /// Helper: create a HashMap of blocks from (id, block) pairs.
    fn make_blocks(list: Vec<(&str, Box<dyn Block>)>) -> HashMap<String, Box<dyn Block>> {
        list.into_iter()
            .map(|(id, b)| (id.to_string(), b))
            .collect()
    }

    fn conn(id: &str, src_block: &str, src_port: &str, tgt_block: &str, tgt_port: &str) -> Connection {
        Connection::new(
            id.into(),
            src_block.into(),
            src_port.into(),
            tgt_block.into(),
            tgt_port.into(),
        )
    }

    // ── Valid graph ─────────────────────────────────────────────────────

    #[test]
    fn test_valid_linear_pipeline() {
        let blocks = make_blocks(vec![
            ("heap", Box::new(HeapFileBlock::new())),
            ("btree", Box::new(BTreeIndexBlock::new())),
        ]);

        let connections = vec![conn("c1", "heap", "stored", "btree", "records")];

        // heap is the entry point (receives data from workload).
        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        assert!(result.valid, "Errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    // ── Missing blocks ──────────────────────────────────────────────────

    #[test]
    fn test_connection_references_missing_block() {
        let blocks = make_blocks(vec![("heap", Box::new(HeapFileBlock::new()))]);

        let connections = vec![conn("c1", "heap", "stored", "missing", "records")];

        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("unknown target block")));
    }

    // ── Port existence ──────────────────────────────────────────────────

    #[test]
    fn test_invalid_port_name() {
        let blocks = make_blocks(vec![
            ("heap", Box::new(HeapFileBlock::new())),
            ("btree", Box::new(BTreeIndexBlock::new())),
        ]);

        let connections = vec![conn("c1", "heap", "nonexistent", "btree", "records")];

        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("no port 'nonexistent'")));
    }

    // ── Port direction ──────────────────────────────────────────────────

    #[test]
    fn test_wrong_port_direction() {
        let blocks = make_blocks(vec![
            ("heap", Box::new(HeapFileBlock::new())),
            ("btree", Box::new(BTreeIndexBlock::new())),
        ]);

        // Connecting input → input (heap's "records" is an input port)
        let connections = vec![conn("c1", "heap", "records", "btree", "records")];

        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("not an output port")));
    }

    // ── Type compatibility ──────────────────────────────────────────────

    #[test]
    fn test_incompatible_port_types() {
        // All our blocks use DataStream so types are compatible.
        let blocks = make_blocks(vec![
            ("heap", Box::new(HeapFileBlock::new())),
            ("btree", Box::new(BTreeIndexBlock::new())),
        ]);

        let connections = vec![conn("c1", "heap", "stored", "btree", "records")];

        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    // ── Cycle detection ─────────────────────────────────────────────────

    #[test]
    fn test_cycle_detected() {
        let blocks = make_blocks(vec![
            ("a", Box::new(HeapFileBlock::new())),
            ("b", Box::new(HeapFileBlock::new())),
        ]);

        // a → b → a  creates a cycle.
        let connections = vec![
            conn("c1", "a", "stored", "b", "records"),
            conn("c2", "b", "stored", "a", "records"),
        ];

        let result = GraphValidator::validate(&blocks, &connections, &["a", "b"]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("cycle")));
    }

    #[test]
    fn test_three_node_cycle() {
        let blocks = make_blocks(vec![
            ("a", Box::new(HeapFileBlock::new())),
            ("b", Box::new(HeapFileBlock::new())),
            ("c", Box::new(HeapFileBlock::new())),
        ]);

        let connections = vec![
            conn("c1", "a", "stored", "b", "records"),
            conn("c2", "b", "stored", "c", "records"),
            conn("c3", "c", "stored", "a", "records"),
        ];

        let result = GraphValidator::validate(&blocks, &connections, &["a", "b", "c"]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("cycle")));
    }

    // ── Duplicate connections ───────────────────────────────────────────

    #[test]
    fn test_duplicate_connection() {
        let blocks = make_blocks(vec![
            ("heap", Box::new(HeapFileBlock::new())),
            ("btree", Box::new(BTreeIndexBlock::new())),
        ]);

        let connections = vec![
            conn("c1", "heap", "stored", "btree", "records"),
            conn("c2", "heap", "stored", "btree", "records"),
        ];

        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("Duplicate")));
    }

    // ── Disconnected block warning ──────────────────────────────────────

    #[test]
    fn test_disconnected_block_warning() {
        let blocks = make_blocks(vec![
            ("heap", Box::new(HeapFileBlock::new())),
            ("btree", Box::new(BTreeIndexBlock::new())),
            ("orphan", Box::new(LRUBufferBlock::new())),
        ]);

        let connections = vec![conn("c1", "heap", "stored", "btree", "records")];

        let result = GraphValidator::validate(&blocks, &connections, &["heap"]);
        // Disconnected blocks are warnings, not errors.
        assert!(result.warnings.iter().any(|w| w.message.contains("orphan")));
    }

    // ── Topological sort ────────────────────────────────────────────────

    #[test]
    fn test_topological_sort_linear() {
        let connections = vec![
            conn("c1", "a", "out", "b", "in"),
            conn("c2", "b", "out", "c", "in"),
        ];

        let order = GraphValidator::topological_sort(&["a", "b", "c"], &connections);
        assert!(order.is_some());
        let order = order.unwrap();
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_topological_sort_diamond() {
        //   a
        //  / \
        // b   c
        //  \ /
        //   d
        let connections = vec![
            conn("c1", "a", "out", "b", "in"),
            conn("c2", "a", "out", "c", "in"),
            conn("c3", "b", "out", "d", "in"),
            conn("c4", "c", "out", "d", "in"),
        ];

        let order = GraphValidator::topological_sort(&["a", "b", "c", "d"], &connections);
        assert!(order.is_some());
        let order = order.unwrap();
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("d"));
        assert!(pos("c") < pos("d"));
    }

    #[test]
    fn test_topological_sort_cycle_returns_none() {
        let connections = vec![
            conn("c1", "a", "out", "b", "in"),
            conn("c2", "b", "out", "a", "in"),
        ];

        let order = GraphValidator::topological_sort(&["a", "b"], &connections);
        assert!(order.is_none());
    }

    // ── Required input ports ────────────────────────────────────────────

    #[test]
    fn test_required_input_not_connected() {
        // BTreeIndex has a required "records" input port — not an entry point.
        let blocks = make_blocks(vec![("btree", Box::new(BTreeIndexBlock::new()))]);

        let connections: Vec<Connection> = vec![];

        let result = GraphValidator::validate(&blocks, &connections, &[]);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("Required input port")));
    }

    #[test]
    fn test_entry_point_skips_required_input_check() {
        // If btree is marked as an entry point, its required inputs are okay.
        let blocks = make_blocks(vec![("btree", Box::new(BTreeIndexBlock::new()))]);

        let result = GraphValidator::validate(&blocks, &[], &["btree"]);
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    // ── Single block (no connections needed, no warnings) ───────────────

    #[test]
    fn test_single_block_no_required_inputs() {
        // HeapFile has a required "records" input. Without entry_points, should fail.
        let blocks = make_blocks(vec![("heap", Box::new(HeapFileBlock::new()))]);
        let result = GraphValidator::validate(&blocks, &[], &[]);
        assert!(!result.valid);
        // But no disconnected-block warning for single block.
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_empty_graph_is_valid() {
        let blocks: HashMap<String, Box<dyn Block>> = HashMap::new();
        let result = GraphValidator::validate(&blocks, &[], &[]);
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }
}
