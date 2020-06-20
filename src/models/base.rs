use crate::models::Data;
use serde::Serialize;

/// Data struct to contain only C section with custom fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Base {
    Data(Data),
}
