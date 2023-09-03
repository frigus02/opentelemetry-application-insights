use std::{borrow::Cow, collections::HashMap, convert::TryInto, str::FromStr};

pub(crate) const DEFAULT_BREEZE_ENDPOINT: &str = "https://dc.services.visualstudio.com";
#[cfg(feature = "live-metrics")]
pub(crate) const DEFAULT_LIVE_ENDPOINT: &str = "https://rt.services.visualstudio.com";
const FIELDS_SEPARATOR: char = ';';
const FIELD_KEY_VALUE_SEPARATOR: char = '=';

#[derive(Debug)]
pub(crate) struct ConnectionString {
    pub(crate) ingestion_endpoint: http::Uri,
    #[cfg(feature = "live-metrics")]
    pub(crate) live_endpoint: http::Uri,
    pub(crate) instrumentation_key: String,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ParseError {
    #[error("invalid format")]
    InvalidFormat,
    #[error("missing instrumentation key")]
    MissingInstrumentationKey,
    #[error("unsupported authorization; only \"ikey\" is supported")]
    UnsupportedAuthorization,
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(http::uri::InvalidUri),
}

impl FromStr for ConnectionString {
    type Err = ParseError;

    /// Parse the given connection string.
    ///
    /// Based on
    /// https://github.com/Azure/azure-sdk-for-js/blob/a4b3762fd7503f90c7bc3bacf9e45ecc4012d3fa/sdk/monitor/monitor-opentelemetry-exporter/src/utils/connectionStringParser.ts
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result: HashMap<String, String> = s
            .split(FIELDS_SEPARATOR)
            .map(|kv| {
                let parts: Vec<&str> = kv.split(FIELD_KEY_VALUE_SEPARATOR).collect();
                if parts.len() == 2 {
                    Ok((parts[0].to_lowercase(), parts[1].to_string()))
                } else {
                    Err(ParseError::InvalidFormat)
                }
            })
            .collect::<Result<_, _>>()?;

        let prefix_suffix = result.remove("endpointsuffix").map(|endpoint_suffix| {
            let location_prefix = result
                .remove("location")
                .map(|x| format!("{}.", x))
                .unwrap_or_else(|| "".into());
            (location_prefix, endpoint_suffix)
        });

        let ingestion_endpoint =
            if let Some(ingestion_endpoint) = result.remove("ingestionendpoint") {
                sanitize_url(ingestion_endpoint)?
            } else if let Some((location_prefix, endpoint_suffix)) = prefix_suffix.as_ref() {
                sanitize_url(format!("https://{}dc.{}", location_prefix, endpoint_suffix))?
            } else {
                http::Uri::from_static(DEFAULT_BREEZE_ENDPOINT)
            };

        #[cfg(feature = "live-metrics")]
        let live_endpoint = if let Some(live_endpoint) = result.remove("liveendpoint") {
            sanitize_url(live_endpoint)?
        } else if let Some((location_prefix, endpoint_suffix)) = prefix_suffix.as_ref() {
            sanitize_url(format!(
                "https://{}live.{}",
                location_prefix, endpoint_suffix
            ))?
        } else {
            http::Uri::from_static(DEFAULT_LIVE_ENDPOINT)
        };

        if let Some(authorization) = result.remove("authorization") {
            if !authorization.eq_ignore_ascii_case("ikey") {
                return Err(ParseError::UnsupportedAuthorization);
            }
        }
        let instrumentation_key = result
            .remove("instrumentationkey")
            .ok_or(ParseError::MissingInstrumentationKey)?;

        Ok(ConnectionString {
            ingestion_endpoint,
            #[cfg(feature = "live-metrics")]
            live_endpoint,
            instrumentation_key,
        })
    }
}

fn sanitize_url(url: String) -> Result<http::Uri, ParseError> {
    let mut new_url: Cow<str> = url.trim().into();
    if !new_url.starts_with("https://") {
        new_url = new_url.replace("http://", "https://").into();
    }

    new_url
        .trim_end_matches('/')
        .try_into()
        .map_err(ParseError::InvalidEndpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;
    use test_case::test_case;

    #[test_case(
        "Authorization=ikey;InstrumentationKey=instr_key;IngestionEndpoint=ingest;LiveEndpoint=live",
        "ingest",
        #[cfg(feature = "live-metrics")] "live",
        "instr_key" ; "default")]
    #[test_case(
        "Authorization=ikey;InstrumentationKey=instr_key;IngestionEndpoint= http://ingest/  ;LiveEndpoint= http://live/  ",
        "https://ingest",
        #[cfg(feature = "live-metrics")] "https://live",
        "instr_key" ; "sanitize url")]
    #[test_case(
        "Foo=1;InstrumentationKey=instr_key;Bar=2;IngestionEndpoint=ingest;LiveEndpoint=live;Baz=3",
        "ingest",
        #[cfg(feature = "live-metrics")] "live",
        "instr_key" ; "ignore unknown fields")]
    #[test_case(
        "InstrumentationKey=instr_key",
        DEFAULT_BREEZE_ENDPOINT,
        #[cfg(feature = "live-metrics")] DEFAULT_LIVE_ENDPOINT,
        "instr_key" ; "default endpoint")]
    #[test_case(
        "InstrumentationKey=instr_key;EndpointSuffix=ai.contoso.com",
        "https://dc.ai.contoso.com",
        #[cfg(feature = "live-metrics")] "https://live.ai.contoso.com",
        "instr_key" ; "endpoint suffix")]
    #[test_case(
        "InstrumentationKey=instr_key;EndpointSuffix=ai.contoso.com;Location=westus2",
        "https://westus2.dc.ai.contoso.com",
        #[cfg(feature = "live-metrics")] "https://westus2.live.ai.contoso.com",
        "instr_key" ; "endpoint suffix & location")]
    #[test_case(
        "InstrumentationKey=instr_key;EndpointSuffix=ai.contoso.com;IngestionEndpoint=ingest;LiveEndpoint=live",
        "ingest",
        #[cfg(feature = "live-metrics")] "live",
        "instr_key" ; "endpoint suffix & override")]
    fn parse_succeeds(
        connection_string: &'static str,
        expected_ingestion_endpoint: &'static str,
        #[cfg(feature = "live-metrics")] expected_live_endpoint: &'static str,
        expected_instrumentation_key: &'static str,
    ) {
        let result: ConnectionString = connection_string.parse().unwrap();
        assert_eq!(
            http::Uri::try_from(expected_ingestion_endpoint).unwrap(),
            result.ingestion_endpoint
        );
        #[cfg(feature = "live-metrics")]
        assert_eq!(
            http::Uri::try_from(expected_live_endpoint).unwrap(),
            result.live_endpoint
        );
        assert_eq!(
            expected_instrumentation_key.to_string(),
            result.instrumentation_key
        );
    }

    #[test_case("Authorization=foo;InstrumentationKey=instr_key" ; "authorization != ikey")]
    #[test_case("InstrumentationKey=instr_key;NoValue" ; "field without value")]
    #[test_case("InstrumentationKey=instr_key;InvalidValue=foo=bar" ; "2 equals signs")]
    #[test_case("IngestionEndpoint=ingest" ; "no instrumentation key")]
    #[test_case("InstrumentationKey=instr_key;IngestionEndpoint=ftp:/foo" ; "invalid endpoint uri")]
    fn parse_fails(connection_string: &'static str) {
        connection_string.parse::<ConnectionString>().unwrap_err();
    }
}
