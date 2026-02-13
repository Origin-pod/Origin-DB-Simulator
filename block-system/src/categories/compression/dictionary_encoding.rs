//! Dictionary Encoding Block
//!
//! Compresses data by mapping repeated values to compact integer codes.
//! Used by columnar databases (Parquet, ClickHouse, Vertica) to compress
//! low-cardinality columns efficiently.
//!
//! ## How it works
//!
//! Build a dictionary mapping each unique value to a small integer.
//! Replace all occurrences of each value with its code. Decompression
//! is a simple lookup. Most effective for low-cardinality columns
//! (e.g., country, status, category).
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `entries_encoded` | Counter | Total records processed |
//! | `dictionary_size` | Gauge | Number of unique values in dictionary |
//! | `compression_ratio` | Gauge | Original size / compressed size |
//! | `dictionary_full_events` | Counter | Times the dictionary reached capacity |

use async_trait::async_trait;
use std::collections::HashMap;

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue};

// ---------------------------------------------------------------------------
// DictionaryEncodingBlock
// ---------------------------------------------------------------------------

pub struct DictionaryEncodingBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    max_dictionary_size: usize,

    // Internal state
    dictionary: HashMap<u64, u32>, // value → code
    next_code: u32,

    // Stats
    entries_encoded: usize,
    dictionary_full_events: usize,
    original_size_bytes: usize,
    compressed_size_bytes: usize,
}

impl DictionaryEncodingBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            max_dictionary_size: 4096,
            dictionary: HashMap::new(),
            next_code: 0,
            entries_encoded: 0,
            dictionary_full_events: 0,
            original_size_bytes: 0,
            compressed_size_bytes: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "dictionary-encoding".into(),
            name: "Dictionary Encoding".into(),
            category: BlockCategory::Compression,
            description: "Compresses low-cardinality data by mapping values to integer codes".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "Dictionary encoding replaces repeated values with compact integer codes. A \
                           dictionary maps each unique value to a small integer code, and the compressed \
                           data stores only codes instead of the original values. Decompression is a \
                           simple dictionary lookup. This is extremely effective for columns with low \
                           cardinality — few distinct values relative to the total number of rows.\n\n\
                           In columnar databases, dictionary encoding is typically the first compression \
                           pass applied to string and enum columns. After replacing strings with integer \
                           codes, a second pass (LZ4, ZSTD, or run-length encoding) can further compress \
                           the integer codes. The combination often achieves 10-100x compression ratios on \
                           real-world data.\n\n\
                           Think of dictionary encoding like abbreviations in a book. Instead of writing \
                           'United States of America' every time it appears, you define 'USA' in a glossary \
                           and use the abbreviation everywhere else. The glossary is the dictionary, the \
                           abbreviation is the code, and the space savings are enormous when the same long \
                           value appears thousands of times."
                    .into(),
                algorithm: "ENCODE(values[]):\n  \
                           dictionary = {}  // value -> code mapping\n  \
                           next_code = 0\n  \
                           encoded = []\n\n  \
                           For each value in values:\n    \
                             If value in dictionary:\n      \
                               encoded.push(dictionary[value])\n    \
                             Else if dictionary.size() < max_dictionary_size:\n      \
                               dictionary[value] = next_code\n      \
                               encoded.push(next_code)\n      \
                               next_code += 1\n    \
                             Else:\n      \
                               // Dictionary full — fall back to uncompressed\n      \
                               encoded.push(value)  // store raw\n      \
                               dictionary_full_events += 1\n\n\
                           DECODE(code):\n  \
                           Return reverse_dictionary[code]  // O(1) array lookup\n\n\
                           COMPRESSION_RATIO:\n  \
                           original_size / compressed_size\n  \
                           E.g., 8-byte strings -> 2-byte codes = 4x compression"
                    .into(),
                complexity: Complexity {
                    time: "O(n) to encode n values — one hash lookup per value".into(),
                    space: "O(d) for the dictionary where d = distinct values, plus O(n) for codes"
                        .into(),
                },
                use_cases: vec![
                    "Columnar stores compress string columns (country, status, category)".into(),
                    "Parquet and ORC file formats use dictionary encoding by default".into(),
                    "ClickHouse's LowCardinality type is dictionary encoding".into(),
                    "Data warehouses compress dimension columns with repeated categorical values".into(),
                    "Network protocol encoding — map repeated header strings to integer codes".into(),
                ],
                tradeoffs: vec![
                    "Excellent for low-cardinality columns (< 10K distinct values)".into(),
                    "Poor for high-cardinality columns (dictionary becomes larger than data)".into(),
                    "Dictionary must fit in memory for fast encoding/decoding".into(),
                    "Often combined with other compression (LZ4, ZSTD) on the codes".into(),
                    "Dictionary overhead is fixed per column segment — amortized over many rows".into(),
                    "Query operations on encoded data can work directly on codes (equality, grouping) without decoding".into(),
                ],
                examples: vec![
                    "Parquet — dictionary encoding is the default first pass; falls back to plain encoding when dictionary exceeds page size".into(),
                    "ClickHouse LowCardinality — wraps any type with dictionary encoding, enables vectorized operations on codes".into(),
                    "Vertica — ENCODING RLE for sorted low-cardinality, ENCODING AUTO selects dictionary encoding automatically".into(),
                    "Apache Arrow — dictionary-encoded arrays for efficient in-memory columnar representation".into(),
                ],
                motivation: "Without compression, columnar databases storing billions of rows would require \
                             enormous amounts of storage. Consider a 'country' column with 2 billion rows but \
                             only 200 distinct country names. Storing the full string 'United States' (13 bytes) \
                             2 billion times wastes ~24 GB. With dictionary encoding, each occurrence is replaced \
                             by a 1-byte code, reducing the column to ~2 GB — a 12x savings.\n\n\
                             Beyond storage savings, dictionary encoding also speeds up queries. Filtering \
                             'WHERE country = US' can compare integer codes instead of strings, which is much \
                             faster. GROUP BY operations can also work on codes, deferring the dictionary lookup \
                             to the final output stage."
                    .into(),
                parameter_guide: HashMap::from([
                    ("max_dictionary_size".into(),
                     "The maximum number of distinct values the dictionary can hold. When this limit is \
                      reached, new values cannot be encoded and are stored uncompressed (fallback). A \
                      larger dictionary (8192-65536) can handle higher cardinality columns but uses more \
                      memory and may reduce compression effectiveness when the dictionary itself becomes \
                      large. A smaller dictionary (16-256) works well for very low cardinality (e.g., \
                      boolean, status, or country columns). Parquet uses a page-size-based limit (~1MB \
                      dictionary per column chunk). Recommended: 4096 for general use; 256-1024 for columns \
                      you know have very low cardinality; 8192-65536 for medium cardinality columns."
                        .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "run-length-encoding".into(),
                        comparison: "Run-length encoding (RLE) replaces consecutive repeated values with a \
                                     single (value, count) pair. It is most effective on sorted data where \
                                     identical values cluster together. Dictionary encoding works regardless \
                                     of value order and is effective whenever there are repeated values anywhere \
                                     in the column. In practice, columnar databases often apply dictionary \
                                     encoding first, then RLE on the resulting sorted codes for maximum \
                                     compression."
                            .into(),
                    },
                    Alternative {
                        block_type: "lz4-compression".into(),
                        comparison: "LZ4 is a general-purpose byte-level compression algorithm that works \
                                     on any data pattern. Dictionary encoding is domain-specific — it exploits \
                                     the knowledge that values repeat exactly. Dictionary encoding typically \
                                     achieves better compression ratios for low-cardinality data and allows \
                                     query operations on compressed data (code comparisons). LZ4 requires \
                                     full decompression before data can be queried. Many systems use both: \
                                     dictionary encoding first, then LZ4 on the codes."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "When does dictionary encoding become counterproductive, and how does a database detect this?".into(),
                    "How can a database execute queries directly on dictionary-encoded data without decompressing?".into(),
                    "What is the relationship between dictionary encoding and the LowCardinality type in ClickHouse?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 4: Compression".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "archive".into(),
            color: "#F97316".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(),
            name: "Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Records to compress using dictionary encoding".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "compressed".into(),
            name: "Compressed Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records with `_dict_code` replacing original values".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "max_dictionary_size".into(),
            name: "Max Dictionary Size".into(),
            param_type: ParameterType::Number,
            description: "Maximum number of distinct values before falling back to uncompressed".into(),
            default_value: ParameterValue::Integer(4096),
            required: false,
            constraints: Some(
                ParameterConstraints::new().with_min(16.0).with_max(65536.0),
            ),
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Slider)
                    .with_step(256.0)
                    .with_unit("entries".into()),
            ),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "entries_encoded".into(),
                name: "Entries Encoded".into(),
                metric_type: MetricType::Counter,
                unit: "records".into(),
                description: "Total records processed through encoding".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "dictionary_size".into(),
                name: "Dictionary Size".into(),
                metric_type: MetricType::Gauge,
                unit: "entries".into(),
                description: "Number of unique values in the dictionary".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "compression_ratio".into(),
                name: "Compression Ratio".into(),
                metric_type: MetricType::Gauge,
                unit: "x".into(),
                description: "Original size divided by compressed size".into(),
                aggregations: vec![AggregationType::Avg],
            },
            MetricDefinition {
                id: "dictionary_full_events".into(),
                name: "Dictionary Full".into(),
                metric_type: MetricType::Counter,
                unit: "events".into(),
                description: "Times the dictionary reached maximum capacity".into(),
                aggregations: vec![AggregationType::Sum],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Encode a value — returns the dictionary code.
    pub fn encode(&mut self, value: u64) -> Option<u32> {
        if let Some(&code) = self.dictionary.get(&value) {
            Some(code)
        } else if self.dictionary.len() < self.max_dictionary_size {
            let code = self.next_code;
            self.dictionary.insert(value, code);
            self.next_code += 1;
            Some(code)
        } else {
            self.dictionary_full_events += 1;
            None // Dictionary full — can't encode.
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size_bytes == 0 {
            return 1.0;
        }
        self.original_size_bytes as f64 / self.compressed_size_bytes as f64
    }
}

impl Default for DictionaryEncodingBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for DictionaryEncodingBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] { &[] }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("max_dictionary_size") {
            self.max_dictionary_size = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("max_dictionary_size must be an integer".into()))?
                as usize;
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context.inputs.get("records").cloned().unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        let mut output_records = Vec::with_capacity(records.len());

        for record in records {
            self.entries_encoded += 1;
            let key = record.get::<u64>("_key").ok().flatten().unwrap_or(0);

            // Original: 8 bytes per key (u64).
            self.original_size_bytes += 8;

            let mut out = record;
            if let Some(code) = self.encode(key) {
                // Compressed: 4 bytes per code (u32) — when dictionary is small,
                // could be even less with varint.
                self.compressed_size_bytes += 4;
                let _ = out.insert("_dict_code".into(), code as usize);
                let _ = out.insert("_dict_encoded".into(), true);
            } else {
                // Dictionary full — pass through uncompressed.
                self.compressed_size_bytes += 8;
                let _ = out.insert("_dict_encoded".into(), false);
            }

            context.metrics.increment("entries_encoded");
            output_records.push(out);
        }

        context.metrics.record("dictionary_size", self.dictionary.len() as f64);
        context.metrics.record("compression_ratio", self.compression_ratio());
        context.metrics.record("dictionary_full_events", self.dictionary_full_events as f64);

        let mut outputs = HashMap::new();
        outputs.insert("compressed".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("entries_encoded".into(), self.entries_encoded as f64);
        metrics_summary.insert("dictionary_size".into(), self.dictionary.len() as f64);
        metrics_summary.insert("compression_ratio".into(), self.compression_ratio());
        metrics_summary.insert("dictionary_full_events".into(), self.dictionary_full_events as f64);

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("records") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => ValidationResult::ok(),
                PortValue::None => ValidationResult::ok().with_warning("No records to compress"),
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("max_dictionary_size".into(), self.max_dictionary_size);
        let _ = state.insert("dictionary_size".into(), self.dictionary.len());
        let _ = state.insert("entries_encoded".into(), self.entries_encoded);
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(m)) = state.get::<usize>("max_dictionary_size") { self.max_dictionary_size = m; }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_basic() {
        let mut enc = DictionaryEncodingBlock::new();

        assert_eq!(enc.encode(100), Some(0));
        assert_eq!(enc.encode(200), Some(1));
        assert_eq!(enc.encode(100), Some(0)); // Same value → same code
        assert_eq!(enc.dictionary.len(), 2);
    }

    #[test]
    fn test_dictionary_full() {
        let mut enc = DictionaryEncodingBlock::new();
        enc.max_dictionary_size = 3;

        assert!(enc.encode(1).is_some());
        assert!(enc.encode(2).is_some());
        assert!(enc.encode(3).is_some());
        assert!(enc.encode(4).is_none()); // Dictionary full
        assert_eq!(enc.dictionary_full_events, 1);
    }

    #[test]
    fn test_compression_ratio() {
        let mut enc = DictionaryEncodingBlock::new();
        enc.original_size_bytes = 800; // 100 × 8 bytes
        enc.compressed_size_bytes = 400; // 100 × 4 bytes
        assert!((enc.compression_ratio() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_metadata() {
        let enc = DictionaryEncodingBlock::new();
        assert_eq!(enc.metadata().id, "dictionary-encoding");
        assert_eq!(enc.metadata().category, BlockCategory::Compression);
    }
}
