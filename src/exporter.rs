use crate::connection_string::ConnectionString;
use opentelemetry_http::HttpClient;
#[cfg(feature = "metrics")]
use opentelemetry_sdk::metrics::reader::{AggregationSelector, DefaultAggregationSelector};
#[cfg(feature = "logs")]
use opentelemetry_sdk::Resource;
use std::{error::Error as StdError, fmt::Debug, sync::Arc};

/// Application Insights span exporter
#[derive(Clone)]
pub struct Exporter<C> {
    pub(crate) client: Arc<C>,
    pub(crate) endpoint: Arc<http::Uri>,
    #[cfg(feature = "live-metrics")]
    pub(crate) live_metrics_endpoint: http::Uri,
    pub(crate) instrumentation_key: String,
    pub(crate) sample_rate: f64,
    #[cfg(feature = "metrics")]
    pub(crate) aggregation_selector: Arc<dyn AggregationSelector>,
    #[cfg(feature = "logs")]
    pub(crate) resource: Resource,
}

impl<C: Debug> Debug for Exporter<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Exporter");
        debug
            .field("client", &self.client)
            .field("endpoint", &self.endpoint)
            .field("instrumentation_key", &self.instrumentation_key)
            .field("sample_rate", &self.sample_rate);
        debug.finish()
    }
}

/// Create a new exporter.
pub fn new_exporter_from_connection_string<C: HttpClient + 'static>(
    connection_string: impl AsRef<str>,
    client: C,
) -> Result<Exporter<C>, Box<dyn StdError + Send + Sync + 'static>> {
    let connection_string: ConnectionString = connection_string.as_ref().parse()?;
    Ok(Exporter {
        client: Arc::new(client),
        endpoint: Arc::new(
            append_v2_track(connection_string.ingestion_endpoint)
                .expect("appending /v2/track should always work"),
        ),
        #[cfg(feature = "live-metrics")]
        live_metrics_endpoint: connection_string.live_endpoint,
        instrumentation_key: connection_string.instrumentation_key,
        sample_rate: 100.0,
        #[cfg(feature = "metrics")]
        aggregation_selector: Arc::new(DefaultAggregationSelector::new()),
        #[cfg(feature = "logs")]
        resource: Resource::empty(),
    })
}

/// Create a new exporter.
///
/// Reads connection string from `APPLICATIONINSIGHTS_CONNECTION_STRING` environment variable.
pub fn new_exporter_from_env<C: HttpClient + 'static>(
    client: C,
) -> Result<Exporter<C>, Box<dyn StdError + Send + Sync + 'static>> {
    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")?;
    new_exporter_from_connection_string(connection_string, client)
}

impl<C> Exporter<C> {
    /// Set sample rate, which is passed through to Application Insights. It should be a value
    /// between 0 and 1 and match the rate given to the sampler.
    ///
    /// Default: 1.0
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        // Application Insights expects the sample rate as a percentage.
        self.sample_rate = sample_rate * 100.0;
        self
    }

    /// Set aggregation selector.
    #[cfg(feature = "metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
    pub fn with_aggregation_selector(
        mut self,
        aggregation_selector: impl AggregationSelector + 'static,
    ) -> Self {
        self.aggregation_selector = Arc::new(aggregation_selector);
        self
    }
}

fn append_v2_track(uri: impl ToString) -> Result<http::Uri, http::uri::InvalidUri> {
    crate::uploader::append_path(uri, "v2/track")
}
