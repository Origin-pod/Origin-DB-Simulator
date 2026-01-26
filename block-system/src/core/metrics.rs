//! Metrics system for tracking block performance and behavior
//!
//! This module provides a comprehensive metrics collection system for blocks,
//! supporting various metric types (counters, gauges, histograms, timing) and
//! aggregation functions (sum, avg, percentiles, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Metric definition describing a metric that a block can collect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    /// Unique identifier for the metric
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of metric (counter, gauge, histogram, timing)
    pub metric_type: MetricType,
    /// Unit of measurement (e.g., "ms", "bytes", "ops")
    pub unit: String,
    /// Description of what this metric measures
    pub description: String,
    /// Supported aggregation types for this metric
    pub aggregations: Vec<AggregationType>,
}

/// Type of metric being collected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Monotonically increasing counter (e.g., total operations)
    Counter,
    /// Point-in-time value that can go up or down (e.g., current memory usage)
    Gauge,
    /// Distribution of values (e.g., request sizes)
    Histogram,
    /// Duration measurements (e.g., operation latency)
    Timing,
}

/// Type of aggregation to apply to collected metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationType {
    /// Sum of all values
    Sum,
    /// Average (mean) of all values
    Avg,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// 50th percentile (median)
    P50,
    /// 95th percentile
    P95,
    /// 99th percentile
    P99,
}

/// Thread-safe metrics collector for runtime metric collection
///
/// This collector stores raw metric values and provides aggregation functions
/// for analyzing the collected data.
pub struct MetricsCollector {
    /// Stores metric values keyed by metric ID
    metrics: Arc<Mutex<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record a metric value
    ///
    /// # Arguments
    /// * `metric_id` - The ID of the metric to record
    /// * `value` - The value to record
    ///
    /// # Examples
    /// ```
    /// use block_system::core::metrics::MetricsCollector;
    ///
    /// let collector = MetricsCollector::new();
    /// collector.record("latency_ms", 42.5);
    /// ```
    pub fn record(&self, metric_id: &str, value: f64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics
            .entry(metric_id.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Increment a counter metric by 1
    ///
    /// This is a convenience method for counter metrics.
    ///
    /// # Arguments
    /// * `metric_id` - The ID of the counter metric to increment
    ///
    /// # Examples
    /// ```
    /// use block_system::core::metrics::MetricsCollector;
    ///
    /// let collector = MetricsCollector::new();
    /// collector.increment("requests_total");
    /// ```
    pub fn increment(&self, metric_id: &str) {
        self.record(metric_id, 1.0);
    }

    /// Get all recorded values for a metric
    ///
    /// # Arguments
    /// * `metric_id` - The ID of the metric
    ///
    /// # Returns
    /// A vector of all recorded values, or an empty vector if the metric doesn't exist
    pub fn get_values(&self, metric_id: &str) -> Vec<f64> {
        let metrics = self.metrics.lock().unwrap();
        metrics.get(metric_id).cloned().unwrap_or_default()
    }

    /// Aggregate metric values using the specified aggregation type
    ///
    /// # Arguments
    /// * `metric_id` - The ID of the metric
    /// * `agg_type` - The type of aggregation to apply
    ///
    /// # Returns
    /// The aggregated value, or None if no values have been recorded
    ///
    /// # Examples
    /// ```
    /// use block_system::core::metrics::{MetricsCollector, AggregationType};
    ///
    /// let collector = MetricsCollector::new();
    /// collector.record("latency_ms", 10.0);
    /// collector.record("latency_ms", 20.0);
    /// collector.record("latency_ms", 30.0);
    ///
    /// assert_eq!(collector.aggregate("latency_ms", AggregationType::Avg), Some(20.0));
    /// assert_eq!(collector.aggregate("latency_ms", AggregationType::Min), Some(10.0));
    /// assert_eq!(collector.aggregate("latency_ms", AggregationType::Max), Some(30.0));
    /// ```
    pub fn aggregate(&self, metric_id: &str, agg_type: AggregationType) -> Option<f64> {
        let values = self.get_values(metric_id);
        if values.is_empty() {
            return None;
        }

        match agg_type {
            AggregationType::Sum => Some(values.iter().sum()),
            AggregationType::Avg => Some(values.iter().sum::<f64>() / values.len() as f64),
            AggregationType::Min => values
                .iter()
                .cloned()
                .min_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::Max => values
                .iter()
                .cloned()
                .max_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::P50 => Self::percentile(&values, 0.5),
            AggregationType::P95 => Self::percentile(&values, 0.95),
            AggregationType::P99 => Self::percentile(&values, 0.99),
        }
    }

    /// Calculate a percentile from a set of values
    ///
    /// Uses linear interpolation between values for more accurate percentile calculation.
    ///
    /// # Arguments
    /// * `values` - Slice of values to calculate percentile from
    /// * `p` - Percentile to calculate (0.0 to 1.0)
    ///
    /// # Returns
    /// The percentile value, or None if values is empty
    fn percentile(values: &[f64], p: f64) -> Option<f64> {
        if values.is_empty() {
            return None;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx = (sorted.len() as f64 - 1.0) * p;
        let idx_lower = idx.floor() as usize;
        let idx_upper = idx.ceil() as usize;

        if idx_lower == idx_upper {
            sorted.get(idx_lower).cloned()
        } else {
            // Linear interpolation between the two nearest values
            let lower = sorted[idx_lower];
            let upper = sorted[idx_upper];
            let fraction = idx - idx_lower as f64;
            Some(lower + (upper - lower) * fraction)
        }
    }

    /// Clear all recorded metrics
    pub fn clear(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.clear();
    }

    /// Get all metric IDs that have recorded values
    pub fn get_metric_ids(&self) -> Vec<String> {
        let metrics = self.metrics.lock().unwrap();
        metrics.keys().cloned().collect()
    }

    /// Get the number of values recorded for a metric
    pub fn get_count(&self, metric_id: &str) -> usize {
        let metrics = self.metrics.lock().unwrap();
        metrics.get(metric_id).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        let metrics = self.metrics.lock().unwrap();
        Self {
            metrics: Arc::new(Mutex::new(metrics.clone())),
        }
    }
}

/// Stub for Logger - will be implemented in a separate module
///
/// This type is used by blocks for logging operations and debug information.
#[derive(Debug, Clone)]
pub struct Logger {
    // Placeholder fields for future implementation
    _private: (),
}

impl Logger {
    /// Create a new logger instance
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Log a debug message (stub)
    pub fn debug(&self, _message: &str) {
        // Stub implementation
    }

    /// Log an info message (stub)
    pub fn info(&self, _message: &str) {
        // Stub implementation
    }

    /// Log a warning message (stub)
    pub fn warn(&self, _message: &str) {
        // Stub implementation
    }

    /// Log an error message (stub)
    pub fn error(&self, _message: &str) {
        // Stub implementation
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub for StorageContext - will be implemented in a separate module
///
/// This type provides access to storage operations for blocks.
#[derive(Debug, Clone)]
pub struct StorageContext {
    // Placeholder fields for future implementation
    _private: (),
}

impl StorageContext {
    /// Create a new storage context
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for StorageContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_get_values() {
        let collector = MetricsCollector::new();
        collector.record("test_metric", 1.0);
        collector.record("test_metric", 2.0);
        collector.record("test_metric", 3.0);

        let values = collector.get_values("test_metric");
        assert_eq!(values, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_increment() {
        let collector = MetricsCollector::new();
        collector.increment("counter");
        collector.increment("counter");
        collector.increment("counter");

        let values = collector.get_values("counter");
        assert_eq!(values, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_aggregation_sum() {
        let collector = MetricsCollector::new();
        collector.record("metric", 1.0);
        collector.record("metric", 2.0);
        collector.record("metric", 3.0);

        assert_eq!(collector.aggregate("metric", AggregationType::Sum), Some(6.0));
    }

    #[test]
    fn test_aggregation_avg() {
        let collector = MetricsCollector::new();
        collector.record("metric", 10.0);
        collector.record("metric", 20.0);
        collector.record("metric", 30.0);

        assert_eq!(collector.aggregate("metric", AggregationType::Avg), Some(20.0));
    }

    #[test]
    fn test_aggregation_min_max() {
        let collector = MetricsCollector::new();
        collector.record("metric", 5.0);
        collector.record("metric", 2.0);
        collector.record("metric", 8.0);
        collector.record("metric", 1.0);

        assert_eq!(collector.aggregate("metric", AggregationType::Min), Some(1.0));
        assert_eq!(collector.aggregate("metric", AggregationType::Max), Some(8.0));
    }

    #[test]
    fn test_percentile_p50() {
        let collector = MetricsCollector::new();
        for i in 1..=100 {
            collector.record("metric", i as f64);
        }

        let p50 = collector.aggregate("metric", AggregationType::P50);
        assert!(p50.is_some());
        let p50_value = p50.unwrap();
        // P50 should be around 50.5 (median of 1-100)
        assert!((p50_value - 50.5).abs() < 0.1);
    }

    #[test]
    fn test_percentile_p95() {
        let collector = MetricsCollector::new();
        for i in 1..=100 {
            collector.record("metric", i as f64);
        }

        let p95 = collector.aggregate("metric", AggregationType::P95);
        assert!(p95.is_some());
        let p95_value = p95.unwrap();
        // P95 should be around 95.05 (95th percentile of 1-100)
        assert!((p95_value - 95.05).abs() < 0.1);
    }

    #[test]
    fn test_percentile_p99() {
        let collector = MetricsCollector::new();
        for i in 1..=100 {
            collector.record("metric", i as f64);
        }

        let p99 = collector.aggregate("metric", AggregationType::P99);
        assert!(p99.is_some());
        let p99_value = p99.unwrap();
        // P99 should be around 99.01 (99th percentile of 1-100)
        assert!((p99_value - 99.01).abs() < 0.1);
    }

    #[test]
    fn test_empty_metric_returns_none() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.aggregate("nonexistent", AggregationType::Sum), None);
        assert_eq!(collector.aggregate("nonexistent", AggregationType::Avg), None);
        assert_eq!(collector.aggregate("nonexistent", AggregationType::P50), None);
    }

    #[test]
    fn test_clear() {
        let collector = MetricsCollector::new();
        collector.record("metric", 1.0);
        collector.record("metric", 2.0);

        collector.clear();

        let values = collector.get_values("metric");
        assert!(values.is_empty());
    }

    #[test]
    fn test_get_metric_ids() {
        let collector = MetricsCollector::new();
        collector.record("metric1", 1.0);
        collector.record("metric2", 2.0);
        collector.record("metric3", 3.0);

        let mut ids = collector.get_metric_ids();
        ids.sort();

        assert_eq!(ids, vec!["metric1", "metric2", "metric3"]);
    }

    #[test]
    fn test_get_count() {
        let collector = MetricsCollector::new();
        collector.record("metric", 1.0);
        collector.record("metric", 2.0);
        collector.record("metric", 3.0);

        assert_eq!(collector.get_count("metric"), 3);
        assert_eq!(collector.get_count("nonexistent"), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let collector = MetricsCollector::new();
        let collector_clone = collector.clone();

        let handle = thread::spawn(move || {
            for i in 0..100 {
                collector_clone.record("concurrent_metric", i as f64);
            }
        });

        for i in 100..200 {
            collector.record("concurrent_metric", i as f64);
        }

        handle.join().unwrap();

        assert_eq!(collector.get_count("concurrent_metric"), 200);
    }

    #[test]
    fn test_percentile_with_single_value() {
        let collector = MetricsCollector::new();
        collector.record("metric", 42.0);

        assert_eq!(collector.aggregate("metric", AggregationType::P50), Some(42.0));
        assert_eq!(collector.aggregate("metric", AggregationType::P95), Some(42.0));
        assert_eq!(collector.aggregate("metric", AggregationType::P99), Some(42.0));
    }

    #[test]
    fn test_percentile_with_two_values() {
        let collector = MetricsCollector::new();
        collector.record("metric", 10.0);
        collector.record("metric", 20.0);

        let p50 = collector.aggregate("metric", AggregationType::P50).unwrap();
        assert_eq!(p50, 15.0); // Should be the average of 10 and 20
    }

    #[test]
    fn test_metric_definition_serialization() {
        let metric = MetricDefinition {
            id: "test_metric".to_string(),
            name: "Test Metric".to_string(),
            metric_type: MetricType::Counter,
            unit: "ops".to_string(),
            description: "A test metric".to_string(),
            aggregations: vec![AggregationType::Sum, AggregationType::Avg],
        };

        let json = serde_json::to_string(&metric).unwrap();
        let deserialized: MetricDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(metric.id, deserialized.id);
        assert_eq!(metric.name, deserialized.name);
        assert_eq!(metric.metric_type, deserialized.metric_type);
    }
}
