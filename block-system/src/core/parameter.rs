//! Parameter system for block configuration
//!
//! This module defines the parameter system that allows blocks to be configured
//! with various values, including validation, constraints, and UI hints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Unique parameter identifier
    pub id: String,
    /// Human-readable parameter name
    pub name: String,
    /// Parameter type
    pub param_type: ParameterType,
    /// Parameter description
    pub description: String,
    /// Default value
    pub default_value: ParameterValue,
    /// Whether this parameter is required
    pub required: bool,
    /// Optional constraints
    pub constraints: Option<ParameterConstraints>,
    /// Optional UI hints
    pub ui_hint: Option<ParameterUIHint>,
}

/// Parameter types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    /// String parameter
    String,
    /// Numeric parameter
    Number,
    /// Boolean parameter
    Boolean,
    /// Enumeration parameter
    Enum,
    /// Object parameter
    Object,
    /// Array parameter
    Array,
}

/// Parameter value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    /// String value
    String(String),
    /// Floating point number
    Number(f64),
    /// Integer number
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<ParameterValue>),
    /// Object with key-value pairs
    Object(HashMap<String, ParameterValue>),
    /// Null value
    Null,
}

impl ParameterValue {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, ParameterValue::Null)
    }

    /// Try to convert to string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ParameterValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to convert to number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            ParameterValue::Number(n) => Some(*n),
            ParameterValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to convert to integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            ParameterValue::Integer(i) => Some(*i),
            ParameterValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    /// Try to convert to boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to convert to array
    pub fn as_array(&self) -> Option<&Vec<ParameterValue>> {
        match self {
            ParameterValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to convert to object
    pub fn as_object(&self) -> Option<&HashMap<String, ParameterValue>> {
        match self {
            ParameterValue::Object(obj) => Some(obj),
            _ => None,
        }
    }
}

impl From<String> for ParameterValue {
    fn from(s: String) -> Self {
        ParameterValue::String(s)
    }
}

impl From<&str> for ParameterValue {
    fn from(s: &str) -> Self {
        ParameterValue::String(s.to_string())
    }
}

impl From<f64> for ParameterValue {
    fn from(n: f64) -> Self {
        ParameterValue::Number(n)
    }
}

impl From<i64> for ParameterValue {
    fn from(i: i64) -> Self {
        ParameterValue::Integer(i)
    }
}

impl From<bool> for ParameterValue {
    fn from(b: bool) -> Self {
        ParameterValue::Boolean(b)
    }
}

/// Parameter constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraints {
    /// Minimum value (for numbers)
    pub min: Option<f64>,
    /// Maximum value (for numbers)
    pub max: Option<f64>,
    /// Regex pattern (for strings)
    pub pattern: Option<String>,
    /// Allowed values (for enums)
    pub allowed_values: Option<Vec<ParameterValue>>,
    /// Minimum length (for strings/arrays)
    pub min_length: Option<usize>,
    /// Maximum length (for strings/arrays)
    pub max_length: Option<usize>,
}

impl ParameterConstraints {
    /// Create a new empty constraints object
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            pattern: None,
            allowed_values: None,
            min_length: None,
            max_length: None,
        }
    }

    /// Set minimum value
    pub fn with_min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value
    pub fn with_max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set allowed values
    pub fn with_allowed_values(mut self, values: Vec<ParameterValue>) -> Self {
        self.allowed_values = Some(values);
        self
    }

    /// Set length constraints
    pub fn with_length_range(mut self, min_length: Option<usize>, max_length: Option<usize>) -> Self {
        self.min_length = min_length;
        self.max_length = max_length;
        self
    }
}

impl Default for ParameterConstraints {
    fn default() -> Self {
        Self::new()
    }
}

/// UI hints for parameter rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterUIHint {
    /// Widget type to use
    pub widget: WidgetType,
    /// Step size (for sliders)
    pub step: Option<f64>,
    /// Unit label
    pub unit: Option<String>,
    /// Help text
    pub help_text: Option<String>,
}

impl ParameterUIHint {
    /// Create a new UI hint with the specified widget
    pub fn new(widget: WidgetType) -> Self {
        Self {
            widget,
            step: None,
            unit: None,
            help_text: None,
        }
    }

    /// Set step size
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Set unit label
    pub fn with_unit(mut self, unit: String) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Set help text
    pub fn with_help_text(mut self, help_text: String) -> Self {
        self.help_text = Some(help_text);
        self
    }
}

/// Widget types for UI rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetType {
    /// Text input field
    Input,
    /// Slider control
    Slider,
    /// Select dropdown
    Select,
    /// Checkbox
    Checkbox,
    /// Multi-line text area
    Textarea,
    /// JSON editor
    JsonEditor,
}

/// Parameter validator trait
pub trait ParameterValidator: Send + Sync {
    /// Validate a parameter value
    ///
    /// # Arguments
    /// * `value` - The value to validate
    /// * `all_params` - All parameter values (for cross-parameter validation)
    fn validate(
        &self,
        value: &ParameterValue,
        all_params: &HashMap<String, ParameterValue>,
    ) -> ValidationResult;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a validation result with an error
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![msg.into()],
            warnings: Vec::new(),
        }
    }

    /// Create a validation result with multiple errors
    pub fn errors(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the validation result
    pub fn with_warning(mut self, msg: impl Into<String>) -> Self {
        self.warnings.push(msg.into());
        self
    }

    /// Add multiple warnings to the validation result
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }

    /// Check if the validation has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the validation has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Merge another validation result into this one
    pub fn merge(mut self, other: ValidationResult) -> Self {
        if !other.valid {
            self.valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self
    }
}
