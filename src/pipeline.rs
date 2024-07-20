use crate::exporter::Exporter;
#[cfg(feature = "live-metrics")]
use crate::quick_pulse::QuickPulseManager;
#[cfg(feature = "metrics")]
use opentelemetry::metrics::{Meter, MeterProvider as _};
use opentelemetry::{global, trace::TracerProvider as _, KeyValue, Value};
use opentelemetry_http::HttpClient;
#[cfg(feature = "logs")]
use opentelemetry_sdk::logs::{self, BatchLogProcessor, LoggerProvider};
#[cfg(feature = "metrics")]
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::{
    runtime::RuntimeChannel,
    trace::{self, BatchSpanProcessor, Tracer, TracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions as semcov;
use std::fmt::Debug;
#[cfg(feature = "metrics")]
use std::time::Duration;

/// Create a new Application Insights exporter pipeline builder
pub fn new_pipeline<C: HttpClient + 'static>(exporter: Exporter<C>) -> Pipeline<C> {
    Pipeline { exporter }
}

/// Application Insights pipeline
#[derive(Debug)]
pub struct Pipeline<C> {
    exporter: Exporter<C>,
}

/// Application Insights traces pipeline
#[derive(Debug)]
pub struct TracesPipeline<C> {
    exporter: Exporter<C>,
    config: Option<trace::Config>,
    batch_config: Option<trace::BatchConfig>,
    #[cfg(feature = "live-metrics")]
    live_metrics: bool,
}

/// Application Insights logs pipeline
#[derive(Debug)]
#[cfg(feature = "logs")]
#[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
pub struct LogsPipeline<C> {
    exporter: Exporter<C>,
    config: Option<logs::Config>,
    batch_config: Option<logs::BatchConfig>,
}

/// Application Insights metrics pipeline
#[derive(Debug)]
#[cfg(feature = "metrics")]
#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
pub struct MetricsPipeline<C> {
    exporter: Exporter<C>,
    interval: Option<Duration>,
    timeout: Option<Duration>,
    resource: Option<Resource>,
}

impl<C> Pipeline<C> {
    /// Configure a pipeling to exporter traces.
    pub fn traces(self) -> TracesPipeline<C> {
        TracesPipeline {
            exporter: self.exporter,
            config: None,
            batch_config: None,
            #[cfg(feature = "live-metrics")]
            live_metrics: false,
        }
    }

    /// Configure a pipeling to exporter logs.
    #[cfg(feature = "logs")]
    #[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
    pub fn logs(self) -> LogsPipeline<C> {
        LogsPipeline {
            exporter: self.exporter,
            config: None,
            batch_config: None,
        }
    }

    /// Configure a pipeling to exporter metrics.
    #[cfg(feature = "metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
    pub fn metrics(self) -> MetricsPipeline<C> {
        MetricsPipeline {
            exporter: self.exporter,
            interval: None,
            timeout: None,
            resource: None,
        }
    }
}

impl<C> TracesPipeline<C>
where
    C: HttpClient + 'static,
{
    /// Set a trace config.
    ///
    /// If there is an existing `Config` the `Resource`s are merged and any other parameters are
    /// overwritten.
    pub fn with_config(self, mut config: trace::Config) -> Self {
        if let Some(old_config) = self.config {
            let merged_resource = old_config.resource.merge(config.resource.clone());
            config = config.with_resource(merged_resource);
        }

        Self {
            config: Some(config),
            ..self
        }
    }

    /// Set a batch config.
    pub fn with_batch_config(self, batch_config: trace::BatchConfig) -> Self {
        Self {
            batch_config: Some(batch_config),
            ..self
        }
    }

    /// Assign the service name under which to group traces by adding a service.name
    /// `sdk::Resource` or overriding a previous setting of it.
    ///
    /// If a `sdk::Config` does not exist on the `PipelineBuilder` one will be created.
    ///
    /// This will be translated, along with the service namespace, to the Cloud Role Name.
    ///
    /// ```
    /// let tracer = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_service_name("my-application")
    ///     .install_simple();
    /// ```
    pub fn with_service_name<T: Into<Value>>(self, name: T) -> Self {
        let new_resource = Resource::new(vec![KeyValue::new(semcov::resource::SERVICE_NAME, name)]);
        let config = if let Some(old_config) = self.config {
            let merged_resource = old_config.resource.merge(&new_resource);
            old_config.with_resource(merged_resource)
        } else {
            trace::Config::default().with_resource(new_resource)
        };

        Self {
            config: Some(config),
            ..self
        }
    }

    /// Enable live metrics.
    #[cfg(feature = "live-metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live-metrics")))]
    pub fn with_live_metrics(self, enable_live_metrics: bool) -> Self {
        Self {
            live_metrics: enable_live_metrics,
            ..self
        }
    }

    /// Build a configured `TracerProvider` with a simple span processor.
    pub fn build_simple(self) -> TracerProvider {
        let mut builder = TracerProvider::builder().with_simple_exporter(self.exporter);
        if let Some(config) = self.config {
            builder = builder.with_config(config);
        }

        builder.build()
    }

    /// Build a configured `TracerProvider` with a batch span processor using the specified
    /// runtime.
    pub fn build_batch<R: RuntimeChannel>(mut self, runtime: R) -> TracerProvider {
        let config = self.config.take();
        #[cfg(feature = "live-metrics")]
        let live_metrics_processor = if self.live_metrics {
            let mut resource = Resource::default();
            if let Some(ref config) = config {
                resource = resource.merge(config.resource.as_ref());
            };
            Some(QuickPulseManager::new(
                self.exporter.client.clone(),
                self.exporter.live_metrics_endpoint.clone(),
                self.exporter.instrumentation_key.clone(),
                resource,
                runtime.clone(),
            ))
        } else {
            None
        };
        let mut builder = TracerProvider::builder();
        #[cfg(feature = "live-metrics")]
        if let Some(live_metrics_processor) = live_metrics_processor {
            builder = builder.with_span_processor(live_metrics_processor);
        }
        builder = builder.with_span_processor(
            BatchSpanProcessor::builder(self.exporter, runtime)
                .with_batch_config(self.batch_config.unwrap_or_default())
                .build(),
        );
        if let Some(config) = config {
            builder = builder.with_config(config);
        }

        builder.build()
    }

    /// Install an Application Insights pipeline with the recommended defaults.
    ///
    /// This registers a global `TracerProvider`. See the `build_simple` function if you don't need
    /// that.
    pub fn install_simple(self) -> Tracer {
        let trace_provider = self.build_simple();
        let tracer = trace_provider
            .tracer_builder("opentelemetry-application-insights")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_schema_url(semcov::SCHEMA_URL)
            .build();
        let _previous_provider = global::set_tracer_provider(trace_provider);
        tracer
    }

    /// Install an Application Insights pipeline with the recommended defaults.
    ///
    /// This registers a global `TracerProvider`. See the `build_simple` function if you don't need
    /// that.
    pub fn install_batch<R: RuntimeChannel>(self, runtime: R) -> Tracer {
        let trace_provider = self.build_batch(runtime);
        let tracer = trace_provider
            .tracer_builder("opentelemetry-application-insights")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_schema_url(semcov::SCHEMA_URL)
            .build();
        let _previous_provider = global::set_tracer_provider(trace_provider);
        tracer
    }
}

#[cfg(feature = "logs")]
#[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
impl<C> LogsPipeline<C>
where
    C: HttpClient + 'static,
{
    /// Set a logs config.
    ///
    /// If there is an existing `Config` the `Resource`s are merged and any other parameters are
    /// overwritten.
    pub fn with_config(self, mut config: logs::Config) -> Self {
        if let Some(old_config) = self.config {
            let merged_resource = old_config.resource.merge(config.resource.clone());
            config = config.with_resource(merged_resource);
        }

        Self {
            config: Some(config),
            ..self
        }
    }

    /// Set a batch config.
    pub fn with_batch_config(self, batch_config: logs::BatchConfig) -> Self {
        Self {
            batch_config: Some(batch_config),
            ..self
        }
    }

    /// Build a configured `LoggerProvider` with a simple log processor.
    pub fn build_simple(self) -> LoggerProvider {
        let mut builder = LoggerProvider::builder().with_simple_exporter(self.exporter);
        if let Some(config) = self.config {
            builder = builder.with_config(config);
        }

        builder.build()
    }

    /// Build a configured `LoggerProvider` with a batch log processor using the specified
    /// runtime.
    pub fn build_batch<R: RuntimeChannel>(self, runtime: R) -> LoggerProvider {
        let processor = BatchLogProcessor::builder(self.exporter, runtime)
            .with_batch_config(self.batch_config.unwrap_or_default())
            .build();
        let mut builder = LoggerProvider::builder().with_log_processor(processor);
        if let Some(config) = self.config {
            builder = builder.with_config(config);
        }

        builder.build()
    }
}

#[cfg(feature = "metrics")]
#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
impl<C> MetricsPipeline<C>
where
    C: HttpClient + 'static,
{
    /// Set an interval.
    pub fn with_interval(self, interval: Duration) -> Self {
        Self {
            interval: Some(interval),
            ..self
        }
    }

    /// Set a timeout.
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
            ..self
        }
    }

    /// Set a resource.
    pub fn with_resource(self, resource: Resource) -> Self {
        Self {
            resource: Some(resource),
            ..self
        }
    }

    /// Build a configured `MeterProvider` using the specified runtime.
    pub fn build<R: RuntimeChannel>(self, runtime: R) -> SdkMeterProvider {
        let mut reader_builder = PeriodicReader::builder(self.exporter, runtime);
        if let Some(interval) = self.interval {
            reader_builder = reader_builder.with_interval(interval);
        }
        if let Some(timeout) = self.timeout {
            reader_builder = reader_builder.with_timeout(timeout);
        }
        let reader = reader_builder.build();
        let mut meter_provider = SdkMeterProvider::builder().with_reader(reader);
        if let Some(resource) = self.resource {
            meter_provider = meter_provider.with_resource(resource);
        }
        meter_provider.build()
    }

    /// Install an Application Insights pipeline with the recommended defaults.
    ///
    /// This registers a global `MeterProvider`. See the `build` function if you don't need that.
    pub fn install<R: RuntimeChannel>(self, runtime: R) -> Meter {
        let meter_provider = self.build(runtime);
        let meter = meter_provider.versioned_meter(
            "opentelemetry-application-insights",
            Some(env!("CARGO_PKG_VERSION")),
            Some(semcov::SCHEMA_URL),
            None,
        );
        global::set_meter_provider(meter_provider);
        meter
    }
}
