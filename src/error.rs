use opentelemetry_sdk::export::ExportError;
use std::{error::Error as StdError, fmt::Debug};

/// Errors that occurred during span export.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Application Insights telemetry data failed to serialize to JSON. Telemetry reporting failed
    /// because of this.
    ///
    /// Note: This is an error in this crate. If you spot this, please open an issue.
    #[error("serializing upload request failed with {0}")]
    UploadSerializeRequest(serde_json::Error),

    /// Application Insights telemetry data failed serialize or compress. Telemetry reporting failed
    /// because of this.
    ///
    /// Note: This is an error in this crate. If you spot this, please open an issue.
    #[error("compressing upload request failed with {0}")]
    UploadCompressRequest(std::io::Error),

    /// Application Insights telemetry response failed to deserialize from JSON.
    ///
    /// Telemetry reporting may have worked. But since we could not look into the response, we
    /// can't be sure.
    ///
    /// Note: This is an error in this crate. If you spot this, please open an issue.
    #[error("deserializing upload response failed with {0}")]
    UploadDeserializeResponse(serde_json::Error),

    /// Could not complete the HTTP request to Application Insights to send telemetry data.
    /// Telemetry reporting failed because of this.
    #[error("sending upload request failed with {0}")]
    UploadConnection(Box<dyn StdError + Send + Sync + 'static>),

    /// Application Insights returned at least one error for the reported telemetry data.
    #[error("upload failed with {0}")]
    Upload(String),

    /// Failed to process span for live metrics.
    #[cfg(feature = "live-metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live-metrics")))]
    #[error("process span for live metrics failed with {0}")]
    QuickPulseProcessSpan(opentelemetry_sdk::runtime::TrySendError),

    /// Failed to stop live metrics.
    #[cfg(feature = "live-metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live-metrics")))]
    #[error("stop live metrics failed with {0}")]
    QuickPulseShutdown(opentelemetry_sdk::runtime::TrySendError),
}

impl ExportError for Error {
    fn exporter_name(&self) -> &'static str {
        "application-insights"
    }
}
