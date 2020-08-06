use crate::models::Data;
use crate::models::Sanitize;
use serde::Serialize;

/// System variables for a telemetry item.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Envelope {
    /// Type name of telemetry data item.
    pub(crate) name: String,

    /// Event date time when telemetry item was created. This is the wall clock time on the client
    /// when the event was generated. There is no guarantee that the client's time is accurate.
    /// This field must be formatted in UTC ISO 8601 format, with a trailing 'Z' character, as
    /// described publicly on https://en.wikipedia.org/wiki/ISO_8601#UTC. Note: the number of
    /// decimal seconds digits provided are variable (and unspecified). Consumers should handle
    /// this, i.e. managed code consumers should not use format 'O' for parsing as it specifies a
    /// fixed length. Example: 2009-06-15T13:45:30.0000000Z.
    pub(crate) time: String,

    /// Sampling rate used in application. This telemetry item represents 1 / sampleRate actual
    /// telemetry items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sample_rate: Option<f64>,

    /// The application's instrumentation key. The key is typically represented as a GUID, but
    /// there are cases when it is not a guid. No code should rely on iKey being a GUID.
    /// Instrumentation key is case insensitive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) i_key: Option<String>,

    /// Key/value collection of context properties. See ContextTagKeys for information on available
    /// properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tags: Option<std::collections::BTreeMap<String, String>>,

    /// Telemetry data item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Data>,
}

impl Sanitize for Envelope {
    fn sanitize(&mut self) {
        self.name.truncate(1024);
        self.time.truncate(64);
        if let Some(i_key) = self.i_key.as_mut() {
            i_key.truncate(40);
        }
    }
}
