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

// TODO: map trace models to this
// https://github.com/microsoft/ApplicationInsights-node.js/blob/84d57aa1565ca8c3dff1e14aa8f63f00b8f87d34/Library/QuickPulseEnvelopeFactory.ts#L55
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuickPulseDocumentProperty {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseMetric {
    pub(crate) name: String,
    pub(crate) value: f32,
    pub(crate) weight: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseEnvelope {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) documents: Vec<QuickPulseDocument>,
    pub(crate) instance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) role_name: Option<String>,
    pub(crate) instrumentation_key: String,
    pub(crate) invariant_version: i32,
    pub(crate) machine_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) metrics: Vec<QuickPulseMetric>,
    pub(crate) stream_id: String,
    pub(crate) timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,
}
