[![Crates.io 0.2.0](https://img.shields.io/crates/v/opentelemetry-application-insights.svg)](https://crates.io/crates/opentelemetry-application-insights)
[![Documentation 0.2.0](https://docs.rs/opentelemetry-application-insights/badge.svg)](https://docs.rs/opentelemetry-application-insights)
[![Workflow Status](https://github.com/frigus02/opentelemetry-application-insights/workflows/CI/badge.svg)](https://github.com/frigus02/opentelemetry-application-insights/actions?query=workflow%3A%22CI%22)

# opentelemetry-application-insights

An [Azure Application Insights](https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview) exporter implementation for [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust).

**Disclaimer**: This is not an official Microsoft product.

## Usage

Configure the exporter:

```rust
use opentelemetry::{global, sdk};

fn init_tracer() {
    let instrumentation_key = "...".to_string();
    let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key);
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .build();
    global::set_provider(provider);
}
```

Then follow the documentation of [opentelemetry](https://github.com/open-telemetry/opentelemetry-rust) to create spans and events.

## Attribute mapping

OpenTelemetry and Application Insights are using different terminology. This crate tries it's best to map OpenTelemetry fields to their correct Application Insights pendant.

- [OpenTelemetry specification: Span](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/api.md#span)
- [Application Insights data model](https://docs.microsoft.com/en-us/azure/azure-monitor/app/data-model)

The OpenTelemetry SpanKind determines the Application Insights telemetry type:

| OpenTelemetry SpanKind           | Application Insights telemetry type |
| -------------------------------- | ----------------------------------- |
| `CLIENT`, `PRODUCER`, `INTERNAL` | Dependency                          |
| `SERVER`, `CONSUMER`             | Request                             |

The Span's list of Events are converted to Trace telemetry.

The Span's status determines the Success field of a Dependency or Request. Success is `true` if the status is `OK`; otherwise `false`.

For `INTERNAL` Spans the Dependency Type is always `"InProc"` and Success is `true`.

The following of the Span's attributes map to special fields in Application Insights (the mapping tries to follow [OpenTelemetry semantic conventions](https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions)).

| OpenTelemetry attribute key              | Application Insights field     |
| ---------------------------------------- | ------------------------------ |
| `enduser.id`                             | Context: Authenticated user id |
| `http.url`                               | Dependency Data                |
| `db.statement`                           | Dependency Data                |
| `http.host`                              | Dependency Target              |
| `net.peer.name`                          | Dependency Target              |
| `db.instance`                            | Dependency Target              |
| `http.status_code`                       | Dependency Result code         |
| `db.type`                                | Dependency Type                |
| `messaging.system`                       | Dependency Type                |
| `"HTTP"` if any `http.` attribute exists | Dependency Type                |
| `"DB"` if any `db.` attribute exists     | Dependency Type                |
| `http.url`                               | Request Url                    |
| `http.target`                            | Request Url                    |
| `http.status_code`                       | Request Response code          |

All other attributes are be directly converted to custom properties.

For Requests the attributes `http.method` and `http.route` override the Name.

## Thanks

This is based on the amazing work by [Denis Molokanov](https://github.com/dmolokanov). Check out the [appinsights](https://github.com/dmolokanov/appinsights-rs) crate, if you want a more direct integration with Application Insights.

## Application Insights integration

The integration is based on resources mentioned here:

> Can I send telemetry to the Application Insights portal?
>
> We recommend you use our SDKs and use the [SDK API](https://docs.microsoft.com/en-us/azure/azure-monitor/app/api-custom-events-metrics). There are variants of the SDK for various [platforms](https://docs.microsoft.com/en-us/azure/azure-monitor/app/platforms). These SDKs handle buffering, compression, throttling, retries, and so on. However, the [ingestion schema](https://github.com/microsoft/ApplicationInsights-dotnet/tree/master/BASE/Schema/PublicSchema) and [endpoint protocol](https://github.com/Microsoft/ApplicationInsights-Home/blob/master/EndpointSpecs/ENDPOINT-PROTOCOL.md) are public.

https://docs.microsoft.com/en-us/azure/azure-monitor/faq#can-i-send-telemetry-to-the-application-insights-portal
