//! Serialization for span links.
//!
//! Application Insights supports receiving span links as JSON in the property `_MS.links`. This
//! does not appear in swagger API definition, yet, as far as I can tell. Compare with the
//! different SDKs:
//!
//! - [type definition in JS exporter](https://github.com/Azure/azure-sdk-for-js/blob/7f1cb9af148b7ed7331107a3e3cffb37e8ef9409/sdk/monitor/monitor-opentelemetry-exporter/src/types.ts#L21-L28)
//! - [serialization in JS exporter](https://github.com/Azure/azure-sdk-for-js/blob/c66cad23c4b803719db65cb48a453b0adc13307b/sdk/monitor/monitor-opentelemetry-exporter/src/utils/spanUtils.ts#L149-L155)
//! - [serialization in Python exporter](https://github.com/Azure/azure-sdk-for-python/blob/aa3a4b32e4d27f15ffd6429cefacce67f5776162/sdk/monitor/azure-monitor-opentelemetry-exporter/azure/monitor/opentelemetry/exporter/export/trace/_exporter.py#L517-L527)

use opentelemetry::trace::Link;

pub(crate) const MS_LINKS_KEY: &str = "_MS.links";

/// Maximum number of links that fit into the property.
///
/// Links are serialized as a JSON array, e.g.
///
/// ```json
/// [{"operation_Id":"77225ad66928295345ea7c9b0a97682e","id":"7c29182f74d01363"}]
/// ```
///
/// Each link is a fixed length of 75 (plus 1 for the comma between links). Property values can be
/// a maximum of 8192 characters. Therefore the maximum number of links is:
///
/// ```plain
/// (8192 - 2) / 76 = 107.76...
/// ```
pub(crate) const MS_LINKS_MAX_LEN: usize = 107;

pub(crate) fn serialize_ms_links(links: &[Link]) -> String {
    let count = links.len().min(MS_LINKS_MAX_LEN);
    let mut res = String::with_capacity(count * 76 + 2);
    res.push('[');
    for link in links.iter().take(MS_LINKS_MAX_LEN) {
        res.push_str(r#"{"operation_Id":""#);
        res.push_str(&link.span_context.trace_id().to_string());
        res.push_str(r#"","id":""#);
        res.push_str(&link.span_context.span_id().to_string());
        res.push_str(r#""},"#);
    }
    if count > 0 {
        res.pop().expect("can remove trailing comma");
    }
    res.push(']');
    res
}
