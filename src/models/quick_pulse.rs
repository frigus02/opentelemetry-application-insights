use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseDocument {
    #[serde(rename = "__type")]
    pub(crate) type_: String,
    #[serde(flatten)]
    pub(crate) document_type: QuickPulseDocumentType,
    pub(crate) version: String,
    pub(crate) operation_id: String,
    pub(crate) properties: Vec<QuickPulseDocumentProperty>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "document_type", rename_all = "PascalCase")]
pub(crate) enum QuickPulseDocumentType {
    Event {
        name: String,
    },
    Exception {
        exception: String,
        exception_message: String,
        exception_type: String,
    },
    Trace {
        message: String,
        severity_level: String,
    },
    Metric,
    Request {
        name: String,
        success: Option<bool>,
        duration: String,
        response_code: String,
        operation_name: String,
    },
    Dependency {
        name: String,
        target: String,
        success: Option<bool>,
        duration: String,
        result_code: String,
        command_name: String,
        dependency_type_name: String,
        operation_name: String,
    },
    Availability,
    PageView,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuickPulseDocumentProperty {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseMetric {
    pub(crate) name: String,
    pub(crate) value: i32,
    pub(crate) weight: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseEnvelope {
    pub(crate) documents: Vec<QuickPulseDocument>,
    pub(crate) instance: String,
    pub(crate) role_name: String,
    pub(crate) instrumentation_key: String,
    pub(crate) invariant_version: i32,
    pub(crate) machine_name: String,
    pub(crate) metrics: Vec<QuickPulseMetric>,
    pub(crate) stream_id: String,
    pub(crate) timestamp: String,
    pub(crate) version: String,
}
