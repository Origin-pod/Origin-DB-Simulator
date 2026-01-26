//! Tests for the parameter system
//!
//! Parameters allow blocks to be configured with various values.
//! This module tests parameter definitions, values, constraints, and validation.

#[cfg(test)]
mod tests {
    use crate::core::parameter::*;
    use std::collections::HashMap;

    /// Test Parameter creation
    ///
    /// Parameters define configurable aspects of blocks
    #[test]
    fn test_parameter_creation() {
        let param = Parameter {
            id: "buffer_size".to_string(),
            name: "Buffer Size".to_string(),
            param_type: ParameterType::Number,
            description: "Size of the buffer in bytes".to_string(),
            default_value: ParameterValue::Integer(1024),
            required: true,
            constraints: Some(
                ParameterConstraints::new()
                    .with_min(0.0)
                    .with_max(10000.0),
            ),
            ui_hint: Some(ParameterUIHint::new(WidgetType::Slider).with_unit("bytes".to_string())),
        };

        assert_eq!(param.id, "buffer_size");
        assert_eq!(param.param_type, ParameterType::Number);
        assert!(param.required);
        assert!(param.constraints.is_some());
        assert!(param.ui_hint.is_some());
    }

    /// Test ParameterValue creation and conversion
    ///
    /// ParameterValues can hold different types of data
    #[test]
    fn test_parameter_value_types() {
        let string_val = ParameterValue::String("test".to_string());
        let int_val = ParameterValue::Integer(42);
        let num_val = ParameterValue::Number(3.14);
        let bool_val = ParameterValue::Boolean(true);
        let null_val = ParameterValue::Null;

        assert_eq!(string_val.as_string(), Some("test"));
        assert_eq!(int_val.as_integer(), Some(42));
        assert_eq!(num_val.as_number(), Some(3.14));
        assert_eq!(bool_val.as_bool(), Some(true));
        assert!(null_val.is_null());
    }

    /// Test ParameterValue array
    #[test]
    fn test_parameter_value_array() {
        let array = ParameterValue::Array(vec![
            ParameterValue::Integer(1),
            ParameterValue::Integer(2),
            ParameterValue::Integer(3),
        ]);

        let arr = array.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_integer(), Some(1));
        assert_eq!(arr[1].as_integer(), Some(2));
        assert_eq!(arr[2].as_integer(), Some(3));
    }

    /// Test ParameterValue object
    #[test]
    fn test_parameter_value_object() {
        let mut obj_map = HashMap::new();
        obj_map.insert("host".to_string(), ParameterValue::String("localhost".to_string()));
        obj_map.insert("port".to_string(), ParameterValue::Integer(5432));

        let object = ParameterValue::Object(obj_map);

        let obj = object.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj.get("host").unwrap().as_string(), Some("localhost"));
        assert_eq!(obj.get("port").unwrap().as_integer(), Some(5432));
    }

    /// Test ParameterValue From implementations
    #[test]
    fn test_parameter_value_from() {
        let from_string: ParameterValue = "hello".into();
        assert_eq!(from_string.as_string(), Some("hello"));

        let from_i64: ParameterValue = 100i64.into();
        assert_eq!(from_i64.as_integer(), Some(100));

        let from_f64: ParameterValue = 2.5f64.into();
        assert_eq!(from_f64.as_number(), Some(2.5));

        let from_bool: ParameterValue = true.into();
        assert_eq!(from_bool.as_bool(), Some(true));
    }

    /// Test ParameterConstraints with min/max
    ///
    /// Constraints limit the valid values for parameters
    #[test]
    fn test_parameter_constraints_range() {
        let constraints = ParameterConstraints::new()
            .with_min(0.0)
            .with_max(100.0);

        assert_eq!(constraints.min, Some(0.0));
        assert_eq!(constraints.max, Some(100.0));
    }

    /// Test ParameterConstraints with allowed values
    ///
    /// Enum parameters have a fixed set of allowed values
    #[test]
    fn test_parameter_constraints_allowed_values() {
        let constraints = ParameterConstraints::new().with_allowed_values(vec![
            ParameterValue::String("small".to_string()),
            ParameterValue::String("medium".to_string()),
            ParameterValue::String("large".to_string()),
        ]);

        let allowed = constraints.allowed_values.unwrap();
        assert_eq!(allowed.len(), 3);
        assert_eq!(allowed[0].as_string(), Some("small"));
        assert_eq!(allowed[1].as_string(), Some("medium"));
        assert_eq!(allowed[2].as_string(), Some("large"));
    }

    /// Test ParameterConstraints with length range
    ///
    /// String and array parameters can have length constraints
    #[test]
    fn test_parameter_constraints_length() {
        let constraints = ParameterConstraints::new().with_length_range(Some(1), Some(100));

        assert_eq!(constraints.min_length, Some(1));
        assert_eq!(constraints.max_length, Some(100));
    }

    /// Test ParameterUIHint
    ///
    /// UI hints help render parameters in the user interface
    #[test]
    fn test_parameter_ui_hint() {
        let hint = ParameterUIHint::new(WidgetType::Slider)
            .with_step(0.1)
            .with_unit("ms".to_string())
            .with_help_text("Adjust the timeout duration".to_string());

        assert_eq!(hint.widget, WidgetType::Slider);
        assert_eq!(hint.step, Some(0.1));
        assert_eq!(hint.unit, Some("ms".to_string()));
        assert_eq!(
            hint.help_text,
            Some("Adjust the timeout duration".to_string())
        );
    }

    /// Test all WidgetType variants
    #[test]
    fn test_widget_types() {
        let widgets = vec![
            WidgetType::Input,
            WidgetType::Slider,
            WidgetType::Select,
            WidgetType::Checkbox,
            WidgetType::Textarea,
            WidgetType::JsonEditor,
        ];

        for widget in widgets {
            let json = serde_json::to_string(&widget).unwrap();
            let deserialized: WidgetType = serde_json::from_str(&json).unwrap();
            assert_eq!(widget, deserialized);
        }
    }

    /// Test ValidationResult success
    #[test]
    fn test_validation_result_ok() {
        let result = ValidationResult::ok();

        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 0);
        assert!(!result.has_errors());
        assert!(!result.has_warnings());
    }

    /// Test ValidationResult with error
    #[test]
    fn test_validation_result_error() {
        let result = ValidationResult::error("Value out of range");

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Value out of range");
        assert!(result.has_errors());
    }

    /// Test ValidationResult with multiple errors
    #[test]
    fn test_validation_result_multiple_errors() {
        let result = ValidationResult::errors(vec![
            "Error 1".to_string(),
            "Error 2".to_string(),
            "Error 3".to_string(),
        ]);

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 3);
        assert!(result.has_errors());
    }

    /// Test ValidationResult with warnings
    #[test]
    fn test_validation_result_with_warnings() {
        let result = ValidationResult::ok()
            .with_warning("Value is close to maximum")
            .with_warning("Consider using a smaller value");

        assert!(result.valid); // Still valid despite warnings
        assert_eq!(result.warnings.len(), 2);
        assert!(result.has_warnings());
    }

    /// Test ValidationResult merge
    ///
    /// Merging allows combining validation results from multiple checks
    #[test]
    fn test_validation_result_merge() {
        let result1 = ValidationResult::ok().with_warning("Warning 1");
        let result2 = ValidationResult::error("Error 1");

        let merged = result1.merge(result2);

        assert!(!merged.valid); // Error makes it invalid
        assert_eq!(merged.errors.len(), 1);
        assert_eq!(merged.warnings.len(), 1);
    }

    /// Test Parameter serialization
    #[test]
    fn test_parameter_serialization() {
        let param = Parameter {
            id: "timeout".to_string(),
            name: "Timeout".to_string(),
            param_type: ParameterType::Number,
            description: "Connection timeout".to_string(),
            default_value: ParameterValue::Number(30.0),
            required: false,
            constraints: Some(ParameterConstraints::new().with_min(1.0).with_max(120.0)),
            ui_hint: Some(ParameterUIHint::new(WidgetType::Slider).with_unit("seconds".to_string())),
        };

        let json = serde_json::to_string(&param).unwrap();
        let deserialized: Parameter = serde_json::from_str(&json).unwrap();

        assert_eq!(param.id, deserialized.id);
        assert_eq!(param.name, deserialized.name);
        assert_eq!(param.param_type, deserialized.param_type);
        assert_eq!(param.required, deserialized.required);
    }

    /// Test ParameterValue serialization
    #[test]
    fn test_parameter_value_serialization() {
        let values = vec![
            ParameterValue::String("test".to_string()),
            ParameterValue::Integer(42),
            ParameterValue::Number(3.14),
            ParameterValue::Boolean(true),
            ParameterValue::Null,
        ];

        for value in values {
            let json = serde_json::to_string(&value).unwrap();
            let _deserialized: ParameterValue = serde_json::from_str(&json).unwrap();
            // Successful round-trip
        }
    }

    /// Example: Complete parameter definition for a database connection
    #[test]
    fn test_complete_parameter_example() {
        let host_param = Parameter {
            id: "db_host".to_string(),
            name: "Database Host".to_string(),
            param_type: ParameterType::String,
            description: "Hostname or IP address of the database server".to_string(),
            default_value: ParameterValue::String("localhost".to_string()),
            required: true,
            constraints: Some(
                ParameterConstraints::new().with_length_range(Some(1), Some(255)),
            ),
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Input)
                    .with_help_text("Enter the database server hostname".to_string()),
            ),
        };

        let port_param = Parameter {
            id: "db_port".to_string(),
            name: "Database Port".to_string(),
            param_type: ParameterType::Number,
            description: "Port number for database connections".to_string(),
            default_value: ParameterValue::Integer(5432),
            required: true,
            constraints: Some(ParameterConstraints::new().with_min(1.0).with_max(65535.0)),
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Input)
                    .with_help_text("Typically 5432 for PostgreSQL, 3306 for MySQL".to_string()),
            ),
        };

        let ssl_param = Parameter {
            id: "use_ssl".to_string(),
            name: "Use SSL".to_string(),
            param_type: ParameterType::Boolean,
            description: "Enable SSL/TLS encryption for connections".to_string(),
            default_value: ParameterValue::Boolean(true),
            required: false,
            constraints: None,
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Checkbox)
                    .with_help_text("Recommended for production environments".to_string()),
            ),
        };

        // All parameters are valid
        assert_eq!(host_param.id, "db_host");
        assert_eq!(port_param.id, "db_port");
        assert_eq!(ssl_param.id, "use_ssl");
    }
}
