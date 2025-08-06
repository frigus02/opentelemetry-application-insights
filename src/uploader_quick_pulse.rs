use crate::{models::QuickPulseEnvelope, uploader::serialize_request_body, Error, HttpClient};
use bytes::Bytes;
use http::{HeaderName, Request, Uri};
use std::{
    convert::TryFrom,
    time::{Duration, SystemTime},
};

// Allow interior mutability. See https://github.com/hyperium/http/issues/599
#[allow(clippy::declare_interior_mutable_const)]
const QPS_TRANSMISSION_TIME: HeaderName = HeaderName::from_static("x-ms-qps-transmission-time");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_STREAM_ID: HeaderName = HeaderName::from_static("x-ms-qps-stream-id");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_MACHINE_NAME: HeaderName = HeaderName::from_static("x-ms-qps-machine-name");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_INSTANCE_NAME: HeaderName = HeaderName::from_static("x-ms-qps-instance-name");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_ROLE_NAME: HeaderName = HeaderName::from_static("x-ms-qps-role-name");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_INVARIANT_VERSION: HeaderName = HeaderName::from_static("x-ms-qps-invariant-version");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_SUBSCRIBED: HeaderName = HeaderName::from_static("x-ms-qps-subscribed");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_REDIRECT: HeaderName = HeaderName::from_static("x-ms-qps-service-endpoint-redirect-v2");
#[allow(clippy::declare_interior_mutable_const)]
const QPS_INTERVAL_HINT: HeaderName =
    HeaderName::from_static("x-ms-qps-service-endpoint-interval-hint");

pub(crate) enum PostOrPing {
    Post,
    Ping,
}

impl std::fmt::Display for PostOrPing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PostOrPing::Post => "post",
            PostOrPing::Ping => "ping",
        })
    }
}

pub(crate) struct QuickPulseResponse {
    pub(crate) should_post: bool,
    pub(crate) redirected_host: Option<http::Uri>,
    pub(crate) polling_interval_hint: Option<std::time::Duration>,
}

pub(crate) async fn send(
    client: &dyn HttpClient,
    endpoint: &Uri,
    post_or_ping: PostOrPing,
    envelope: QuickPulseEnvelope,
) -> Result<QuickPulseResponse, Error> {
    let payload = serialize_envelope(&envelope, &post_or_ping)?;

    let mut request_builder = Request::post(endpoint)
        .header(http::header::EXPECT, "100-continue")
        .header(
            QPS_TRANSMISSION_TIME,
            quick_pulse_transmission_time(SystemTime::now()),
        )
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_ENCODING, "gzip");
    if matches!(post_or_ping, PostOrPing::Ping) {
        request_builder = request_builder
            .header(QPS_STREAM_ID, envelope.stream_id)
            .header(QPS_MACHINE_NAME, envelope.machine_name)
            .header(QPS_INSTANCE_NAME, envelope.instance)
            .header(QPS_INVARIANT_VERSION, envelope.invariant_version);
        if let Some(role_name) = envelope.role_name {
            request_builder = request_builder.header(QPS_ROLE_NAME, role_name);
        }
    }

    let request = request_builder
        .body(Bytes::from(payload))
        .expect("request should be valid");

    let response = client
        .send_bytes(request)
        .await
        .map_err(Error::UploadConnection)?;

    if response.status().is_success() {
        let should_post = response
            .headers()
            .get(QPS_SUBSCRIBED)
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);
        let redirected_host = response
            .headers()
            .get(QPS_REDIRECT)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| Uri::try_from(v).ok());
        let polling_interval_hint = response
            .headers()
            .get(QPS_INTERVAL_HINT)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis);
        Ok(QuickPulseResponse {
            should_post,
            redirected_host,
            polling_interval_hint,
        })
    } else {
        Err(Error::Upload {
            status_code: response.status().as_u16(),
            can_retry: false,
        })
    }
}

fn serialize_envelope(
    envelope: &QuickPulseEnvelope,
    post_or_ping: &PostOrPing,
) -> Result<Vec<u8>, Error> {
    let serialized = match post_or_ping {
        PostOrPing::Post => serde_json::to_vec(&[&envelope]),
        PostOrPing::Ping => serde_json::to_vec(&envelope),
    }
    .map_err(Error::UploadSerializeRequest)?;
    serialize_request_body(&serialized)
}

/// Time the request was made.
///
/// Expressed as the number of 100-nanosecond intervals elapsed since 12:00 midnight, January 1, 0001.
///
/// .NET uses System.DateTimeOffset.Ticks:
///
/// - https://github.com/microsoft/ApplicationInsights-dotnet/blob/de66d679ff32f5a74553edbf52b10b9dc57ded70/WEB/Src/PerformanceCollector/Perf.Shared.NetStandard/Implementation/QuickPulse/QuickPulseServiceClient.cs#L399
/// - https://learn.microsoft.com/en-us/dotnet/api/system.datetimeoffset?view=net-7.0
///
/// Node.js uses Date.now():
///
/// - https://github.com/microsoft/ApplicationInsights-node.js/blob/11c70daa206d2d225a7c6c8d2d05e98c5c4cc8d0/Library/QuickPulseUtil.ts#L10
fn quick_pulse_transmission_time(now: SystemTime) -> String {
    let nanos_between_0001_and_epoch = 62135596800000000000u128;
    let nanos_since_epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    ((nanos_between_0001_and_epoch + nanos_since_epoch) / 100).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(SystemTime::UNIX_EPOCH + Duration::from_secs(1596665700), "637322625000000000")]
    #[test_case(SystemTime::UNIX_EPOCH + Duration::from_secs(1596665701), "637322625010000000")]
    fn transmission_time(now: SystemTime, expected: &'static str) {
        assert_eq!(expected.to_string(), quick_pulse_transmission_time(now));
    }
}
