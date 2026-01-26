//! Port system for block connections
//!
//! This module defines the port system that enables data flow between blocks,
//! including port definitions, schemas, values, and connection management.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::parameter::ValidationResult;

/// Port definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    /// Unique port identifier
    pub id: String,
    /// Human-readable port name
    pub name: String,
    /// Port data type
    pub port_type: PortType,
    /// Port direction (input or output)
    pub direction: PortDirection,
    /// Whether this port is required
    pub required: bool,
    /// Whether this port accepts multiple connections
    pub multiple: bool,
    /// Port description
    pub description: String,
    /// Optional schema for validation
    pub schema: Option<PortSchema>,
}

/// Port direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDirection {
    /// Input port
    Input,
    /// Output port
    Output,
}

/// Port data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    // Data types
    /// Stream of records
    DataStream,
    /// Single value
    SingleValue,
    /// Batch of records
    Batch,

    // Control signals
    /// Control signal
    Signal,
    /// Transaction context
    Transaction,

    // Metadata
    /// Schema information
    Schema,
    /// Statistics data
    Statistics,

    // Configuration
    /// Configuration object
    Config,
}

/// Port schema for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PortSchema {
    /// Object schema with properties
    Object {
        /// Property schemas
        properties: HashMap<String, Box<PortSchema>>,
        /// Required properties
        required: Vec<String>,
    },
    /// Array schema
    Array {
        /// Schema for array items
        items: Box<PortSchema>,
    },
    /// Primitive type schema
    Primitive {
        /// Primitive type
        prim_type: PrimitiveType,
    },
}

/// Primitive types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// Integer number
    Integer,
    /// Floating point number
    Float,
    /// String
    String,
    /// Boolean
    Boolean,
    /// Byte array
    Bytes,
}

/// Port value - actual data flowing through ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortValue {
    /// Stream of records
    Stream(Vec<Record>),
    /// Single record
    Single(Record),
    /// Batch of records
    Batch(Vec<Record>),
    /// Control signal
    Signal(SignalValue),
    /// No value
    None,
}

impl PortValue {
    /// Check if the value is empty/none
    pub fn is_none(&self) -> bool {
        matches!(self, PortValue::None)
    }

    /// Get the number of records in the value
    pub fn len(&self) -> usize {
        match self {
            PortValue::Stream(records) => records.len(),
            PortValue::Single(_) => 1,
            PortValue::Batch(records) => records.len(),
            PortValue::Signal(_) => 0,
            PortValue::None => 0,
        }
    }

    /// Check if the value contains no records
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// Record data as key-value pairs
    pub data: HashMap<String, JsonValue>,
}

impl Record {
    /// Create a new empty record
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Create a record from a HashMap
    pub fn from_map(data: HashMap<String, JsonValue>) -> Self {
        Self { data }
    }

    /// Insert a field into the record
    pub fn insert<T: Serialize>(&mut self, key: String, value: T) -> Result<(), serde_json::Error> {
        let json_value = serde_json::to_value(value)?;
        self.data.insert(key, json_value);
        Ok(())
    }

    /// Get a field from the record
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, serde_json::Error> {
        match self.data.get(key) {
            Some(value) => {
                let result = serde_json::from_value(value.clone())?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
}

impl Default for Record {
    fn default() -> Self {
        Self::new()
    }
}

/// Signal values for control flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalValue {
    /// Start signal
    Start,
    /// Stop signal
    Stop,
    /// Commit transaction
    Commit,
    /// Abort transaction
    Abort,
    /// Custom signal
    Custom(String),
}

/// Connection between ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Unique connection identifier
    pub id: String,
    /// Source block ID
    pub source_block_id: String,
    /// Source port ID
    pub source_port_id: String,
    /// Target block ID
    pub target_block_id: String,
    /// Target port ID
    pub target_port_id: String,
    /// Enable backpressure
    pub backpressure: bool,
    /// Optional buffer size
    pub buffer_size: Option<usize>,
}

impl Connection {
    /// Create a new connection
    pub fn new(
        id: String,
        source_block_id: String,
        source_port_id: String,
        target_block_id: String,
        target_port_id: String,
    ) -> Self {
        Self {
            id,
            source_block_id,
            source_port_id,
            target_block_id,
            target_port_id,
            backpressure: false,
            buffer_size: None,
        }
    }

    /// Enable backpressure with optional buffer size
    pub fn with_backpressure(mut self, buffer_size: Option<usize>) -> Self {
        self.backpressure = true;
        self.buffer_size = buffer_size;
        self
    }
}

/// Port validator trait
pub trait PortValidator: Send + Sync {
    /// Validate a port value against the port's schema
    fn validate(&self, value: &PortValue) -> ValidationResult;
}
