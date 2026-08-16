//! Distributed tracing implementation
//!
//! This module provides distributed tracing support using OpenTelemetry
//! and integrates with tracing systems like Jaeger and Zipkin.

use opentelemetry::{
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, Tracer},
        Resource,
    },
    trace::{TraceError, TracerProvider},
};
use std::sync::Arc;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
// Note: OpenTelemetry dependencies are optional and require feature flags
// For now, this provides the interface. Enable with tracing feature.
use std::sync::Arc;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

#[cfg(feature = "tracing")]
use opentelemetry::{
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, Tracer},
        Resource,
    },
    trace::{TraceError, TracerProvider},
};

#[cfg(feature = "tracing")]
use opentelemetry_otlp::WithExportConfig;

#[cfg(feature = "tracing")]
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub enabled: bool,
    pub sampling_rate: f64,
    pub jaeger_endpoint: Option<String>,
    pub zipkin_endpoint: Option<String>,
    pub otlp_endpoint: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "gaussmeridian".to_string(),
            enabled: false,
            sampling_rate: 0.1,
            jaeger_endpoint: None,
            zipkin_endpoint: None,
            otlp_endpoint: None,
        }
    }
}

/// Tracing manager
pub struct TracingManager {
    config: TracingConfig,
    #[cfg(feature = "tracing")]
    tracer: Option<Arc<Tracer>>,
}

impl TracingManager {
    /// Create a new tracing manager
    #[cfg(feature = "tracing")]
    pub fn new(config: TracingConfig) -> Result<Self, opentelemetry::trace::TraceError> {
        let tracer = if config.enabled {
            Some(Arc::new(Self::init_tracer(&config)?))
        } else {
            None
        };

        Ok(Self { config, tracer })
    }

    #[cfg(not(feature = "tracing"))]
    pub fn new(config: TracingConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { config })
    }

    #[cfg(feature = "tracing")]
    fn init_tracer(config: &TracingConfig) -> Result<Tracer, opentelemetry::trace::TraceError> {
        let resource = Resource::new(vec![SERVICE_NAME.string(config.service_name.clone())]);

        // Use OTLP exporter if endpoint is configured, otherwise use stdout
        let tracer = if let Some(ref otlp_endpoint) = config.otlp_endpoint {
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(otlp_endpoint),
                )
                .with_trace_config(
                    trace::config()
                        .with_resource(resource)
                        .with_sampler(trace::Sampler::TraceIdRatioBased(config.sampling_rate)),
                )
                .install_batch(opentelemetry::runtime::Tokio)
                .map_err(|e| {
                    TraceError::Other(format!("Failed to initialize OTLP tracer: {}", e))
                })?
        } else if let Some(ref jaeger_endpoint) = config.jaeger_endpoint {
            // Jaeger exporter (using OTEL collector or direct)
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(jaeger_endpoint),
                )
                .with_trace_config(
                    trace::config()
                        .with_resource(resource)
                        .with_sampler(trace::Sampler::TraceIdRatioBased(config.sampling_rate)),
                )
                .install_batch(opentelemetry::runtime::Tokio)
                .map_err(|e| {
                    TraceError::Other(format!("Failed to initialize Jaeger tracer: {}", e))
                })?
        } else if let Some(ref zipkin_endpoint) = config.zipkin_endpoint {
            // Zipkin exporter
            opentelemetry_zipkin::new_pipeline()
                .with_service_name(&config.service_name)
                .with_collector_endpoint(zipkin_endpoint)
                .install_batch(opentelemetry::runtime::Tokio)
                .map_err(|e| {
                    TraceError::Other(format!("Failed to initialize Zipkin tracer: {}", e))
                })?
        } else {
            // Default to stdout for development
            opentelemetry_stdout::new_pipeline()
                .install_simple()
                .map_err(|e| {
                    TraceError::Other(format!("Failed to initialize stdout tracer: {}", e))
                })?
        };

        // Install trace context propagator
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

        Ok(tracer)
    }

    #[cfg(not(feature = "tracing"))]
    fn init_tracer(_config: &TracingConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Initialize global tracing subscriber
    pub fn init_subscriber(&self) -> Result<(), Box<dyn std::error::Error>> {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let fmt_layer = fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_line_number(true)
            .json();

        #[cfg(feature = "tracing")]
        {
            if let Some(ref tracer) = self.tracer {
                let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());

                let subscriber = Registry::default()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(telemetry_layer);

                set_global_default(subscriber).map_err(|e| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to set global subscriber: {}", e),
                    ))?
                })?;
                return Ok(());
            }
        }

        let subscriber = Registry::default().with(env_filter).with(fmt_layer);

        set_global_default(subscriber).map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to set global subscriber: {}", e),
            ))?
        })?;

        Ok(())
    }

    /// Get the tracer instance
    #[cfg(feature = "tracing")]
    pub fn tracer(&self) -> Option<&Arc<Tracer>> {
        self.tracer.as_ref()
    }
}

impl Drop for TracingManager {
    fn drop(&mut self) {
        #[cfg(feature = "tracing")]
        if self.config.enabled {
            opentelemetry::global::shutdown_tracer_provider();
        }
    }
}

/// Helper function to create tracing spans with consistent attributes
pub fn create_span(name: &str, attributes: &[(&str, &str)]) -> tracing::Span {
    let span = tracing::span!(tracing::Level::INFO, name);

    for (key, value) in attributes {
        span.record(key, value);
    }

    span
}

/// Helper function to add span attributes
pub fn add_span_attributes(attributes: &[(&str, &str)]) {
    let span = tracing::Span::current();
    for (key, value) in attributes {
        span.record(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "gaussmeridian");
        assert!(!config.enabled);
        assert_eq!(config.sampling_rate, 0.1);
    }

    #[tokio::test]
    async fn test_tracing_manager_disabled() {
        let config = TracingConfig {
            enabled: false,
            ..Default::default()
        };

        #[cfg(feature = "tracing")]
        {
            let manager = TracingManager::new(config).unwrap();
            assert!(manager.tracer().is_none());
        }

        #[cfg(not(feature = "tracing"))]
        {
            let manager = TracingManager::new(config).unwrap();
            // Manager created successfully without tracing feature
            assert!(true);
        }
    }
}
