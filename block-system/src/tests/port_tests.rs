//! Tests for the port system
//!
//! Ports enable data flow between blocks. This module tests port definitions,
//! port values, records, and connections.

#[cfg(test)]
mod tests {
    use crate::core::port::*;
    use serde::Serialize;
    use std::collections::HashMap;

    /// Test Port creation
    ///
    /// Ports define how data flows into and out of blocks
    #[test]
    fn test_port_creation() {
        let port = Port {
            id: "input1".to_string(),
            name: "Data Input".to_string(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Primary data input".to_string(),
            schema: None,
        };

        assert_eq!(port.id, "input1");
        assert_eq!(port.direction, PortDirection::Input);
        assert!(port.required);
        assert!(!port.multiple);
    }

    /// Test PortValue with single record
    ///
    /// PortValues wrap the actual data flowing through ports
    #[test]
    fn test_port_value_single_record() {
        let mut record = Record::new();
        record.insert("id".to_string(), 42).unwrap();
        record.insert("name".to_string(), "Alice").unwrap();

        let port_value = PortValue::Single(record);

        assert_eq!(port_value.len(), 1);
        assert!(!port_value.is_empty());
        assert!(!port_value.is_none());
    }

    /// Test PortValue with stream of records
    ///
    /// Streams are collections of records flowing through the system
    #[test]
    fn test_port_value_stream() {
        let mut records = Vec::new();

        for i in 0..5 {
            let mut record = Record::new();
            record.insert("id".to_string(), i).unwrap();
            record.insert("value".to_string(), i * 10).unwrap();
            records.push(record);
        }

        let port_value = PortValue::Stream(records);

        assert_eq!(port_value.len(), 5);
        assert!(!port_value.is_empty());
    }

    /// Test PortValue batch
    ///
    /// Batches are groups of records processed together
    #[test]
    fn test_port_value_batch() {
        let mut batch = Vec::new();

        let mut record1 = Record::new();
        record1.insert("id".to_string(), 1).unwrap();
        batch.push(record1);

        let mut record2 = Record::new();
        record2.insert("id".to_string(), 2).unwrap();
        batch.push(record2);

        let port_value = PortValue::Batch(batch);

        assert_eq!(port_value.len(), 2);
    }

    /// Test PortValue signal
    ///
    /// Signals are control messages for coordinating blocks
    #[test]
    fn test_port_value_signal() {
        let start_signal = PortValue::Signal(SignalValue::Start);
        let stop_signal = PortValue::Signal(SignalValue::Stop);
        let commit_signal = PortValue::Signal(SignalValue::Commit);

        assert_eq!(start_signal.len(), 0);
        assert_eq!(stop_signal.len(), 0);
        assert_eq!(commit_signal.len(), 0);
    }

    /// Test Record creation and manipulation
    ///
    /// Records are key-value pairs representing data rows
    #[test]
    fn test_record_operations() {
        let mut record = Record::new();

        // Insert values of different types
        record.insert("id".to_string(), 100).unwrap();
        record.insert("name".to_string(), "Bob").unwrap();
        record.insert("active".to_string(), true).unwrap();
        record.insert("score".to_string(), 95.5).unwrap();

        // Retrieve values
        let id: Option<i32> = record.get("id").unwrap();
        assert_eq!(id, Some(100));

        let name: Option<String> = record.get("name").unwrap();
        assert_eq!(name, Some("Bob".to_string()));

        let active: Option<bool> = record.get("active").unwrap();
        assert_eq!(active, Some(true));

        let score: Option<f64> = record.get("score").unwrap();
        assert_eq!(score, Some(95.5));
    }

    /// Test Record creation from HashMap
    #[test]
    fn test_record_from_map() {
        let mut data = HashMap::new();
        data.insert("x".to_string(), serde_json::json!(10));
        data.insert("y".to_string(), serde_json::json!(20));

        let record = Record::from_map(data);

        let x: Option<i32> = record.get("x").unwrap();
        let y: Option<i32> = record.get("y").unwrap();

        assert_eq!(x, Some(10));
        assert_eq!(y, Some(20));
    }

    /// Test Connection creation
    ///
    /// Connections link output ports of one block to input ports of another
    #[test]
    fn test_connection_creation() {
        let connection = Connection::new(
            "conn1".to_string(),
            "block1".to_string(),
            "output1".to_string(),
            "block2".to_string(),
            "input1".to_string(),
        );

        assert_eq!(connection.id, "conn1");
        assert_eq!(connection.source_block_id, "block1");
        assert_eq!(connection.source_port_id, "output1");
        assert_eq!(connection.target_block_id, "block2");
        assert_eq!(connection.target_port_id, "input1");
        assert!(!connection.backpressure);
        assert!(connection.buffer_size.is_none());
    }

    /// Test Connection with backpressure
    ///
    /// Backpressure prevents overwhelming downstream blocks
    #[test]
    fn test_connection_with_backpressure() {
        let connection = Connection::new(
            "conn1".to_string(),
            "block1".to_string(),
            "output1".to_string(),
            "block2".to_string(),
            "input1".to_string(),
        )
        .with_backpressure(Some(1000));

        assert!(connection.backpressure);
        assert_eq!(connection.buffer_size, Some(1000));
    }

    /// Test PortSchema for validation
    ///
    /// Schemas define the structure of data flowing through ports
    #[test]
    fn test_port_schema_primitive() {
        let schema = PortSchema::Primitive {
            prim_type: PrimitiveType::Integer,
        };

        // Schema is created successfully
        match schema {
            PortSchema::Primitive { prim_type } => {
                assert_eq!(prim_type, PrimitiveType::Integer);
            }
            _ => panic!("Expected Primitive schema"),
        }
    }

    /// Test PortSchema for objects
    #[test]
    fn test_port_schema_object() {
        let mut properties = HashMap::new();
        properties.insert(
            "id".to_string(),
            Box::new(PortSchema::Primitive {
                prim_type: PrimitiveType::Integer,
            }),
        );
        properties.insert(
            "name".to_string(),
            Box::new(PortSchema::Primitive {
                prim_type: PrimitiveType::String,
            }),
        );

        let schema = PortSchema::Object {
            properties,
            required: vec!["id".to_string()],
        };

        match schema {
            PortSchema::Object { properties, required } => {
                assert_eq!(properties.len(), 2);
                assert_eq!(required.len(), 1);
                assert_eq!(required[0], "id");
            }
            _ => panic!("Expected Object schema"),
        }
    }

    /// Test PortSchema for arrays
    #[test]
    fn test_port_schema_array() {
        let schema = PortSchema::Array {
            items: Box::new(PortSchema::Primitive {
                prim_type: PrimitiveType::Float,
            }),
        };

        match schema {
            PortSchema::Array { items } => match *items {
                PortSchema::Primitive { prim_type } => {
                    assert_eq!(prim_type, PrimitiveType::Float);
                }
                _ => panic!("Expected Primitive items"),
            },
            _ => panic!("Expected Array schema"),
        }
    }

    /// Test SignalValue variants
    ///
    /// Signals coordinate control flow between blocks
    #[test]
    fn test_signal_values() {
        let start = SignalValue::Start;
        let stop = SignalValue::Stop;
        let commit = SignalValue::Commit;
        let abort = SignalValue::Abort;
        let custom = SignalValue::Custom("checkpoint".to_string());

        // All signals should serialize
        let _start_json = serde_json::to_string(&start).unwrap();
        let _stop_json = serde_json::to_string(&stop).unwrap();
        let _commit_json = serde_json::to_string(&commit).unwrap();
        let _abort_json = serde_json::to_string(&abort).unwrap();
        let _custom_json = serde_json::to_string(&custom).unwrap();
    }

    /// Test PortType variants
    #[test]
    fn test_port_types() {
        let types = vec![
            PortType::DataStream,
            PortType::SingleValue,
            PortType::Batch,
            PortType::Signal,
            PortType::Transaction,
            PortType::Schema,
            PortType::Statistics,
            PortType::Config,
        ];

        // All types should serialize/deserialize
        for port_type in types {
            let json = serde_json::to_string(&port_type).unwrap();
            let deserialized: PortType = serde_json::from_str(&json).unwrap();
            assert_eq!(port_type, deserialized);
        }
    }

    /// Test Port serialization
    #[test]
    fn test_port_serialization() {
        let port = Port {
            id: "data_in".to_string(),
            name: "Data Input".to_string(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Main data input".to_string(),
            schema: Some(PortSchema::Primitive {
                prim_type: PrimitiveType::Integer,
            }),
        };

        let json = serde_json::to_string(&port).unwrap();
        let deserialized: Port = serde_json::from_str(&json).unwrap();

        assert_eq!(port.id, deserialized.id);
        assert_eq!(port.name, deserialized.name);
        assert_eq!(port.port_type, deserialized.port_type);
        assert_eq!(port.direction, deserialized.direction);
    }

    /// Test Connection serialization
    #[test]
    fn test_connection_serialization() {
        let connection = Connection::new(
            "conn1".to_string(),
            "source".to_string(),
            "out1".to_string(),
            "target".to_string(),
            "in1".to_string(),
        )
        .with_backpressure(Some(500));

        let json = serde_json::to_string(&connection).unwrap();
        let deserialized: Connection = serde_json::from_str(&json).unwrap();

        assert_eq!(connection.id, deserialized.id);
        assert_eq!(connection.source_block_id, deserialized.source_block_id);
        assert_eq!(connection.buffer_size, deserialized.buffer_size);
    }
}
