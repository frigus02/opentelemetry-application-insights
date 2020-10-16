[![Crates.io 0.4.0](https://img.shields.io/crates/v/opentelemetry-application-insights.svg)](https://crates.io/crates/opentelemetry-application-insights)
[![Documentation 0.4.0](https://docs.rs/opentelemetry-application-insights/badge.svg)](https://docs.rs/opentelemetry-application-insights)
[![Workflow Status](https://github.com/frigus02/opentelemetry-application-insights/workflows/CI/badge.svg)](https://github.com/frigus02/opentelemetry-application-insights/actions?query=workflow%3A%22CI%22)

# opentelemetry-application-insights

An [Azure Application Insights] exporter implementation for [OpenTelemetry Rust].

[Azure Application Insights]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview
[OpenTelemetry Rust]: https://github.com/open-telemetry/opentelemetry-rust

**Disclaimer**: This is not an official Microsoft product.

## Usage

Configure a OpenTelemetry pipeline using the Application Insights exporter and start creating
spans:

```rust
use opentelemetry::{api::trace::Tracer as _, sdk::trace::Tracer};
use opentelemetry_application_insights::Uninstall;

fn init_tracer() -> (Tracer, Uninstall)  {
    let instrumentation_key = "...".to_string();
    opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .install()
}

fn main() {
    let (tracer, _uninstall) = init_tracer();
    tracer.in_span("main", |_cx| {});
}
```

The functions `build` and `install` automatically configure an asynchronous batch exporter if
you enable either the `async-std` or `tokio` feature for the `opentelemetry` crate. Otherwise
spans will be exported synchronously.

## Attribute mapping

OpenTelemetry and Application Insights are using different terminology. This crate tries it's
best to map OpenTelemetry fields to their correct Application Insights pendant.

- [OpenTelemetry specification: Span](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/api.md#span)
- [Application Insights data model](https://docs.microsoft.com/en-us/azure/azure-monitor/app/data-model)

### Spans

The OpenTelemetry SpanKind determines the Application Insights telemetry type:

| OpenTelemetry SpanKind           | Application Insights telemetry type |
| -------------------------------- | ----------------------------------- |
| `CLIENT`, `PRODUCER`, `INTERNAL` | Dependency                          |
| `SERVER`, `CONSUMER`             | Request                             |

The Span's status determines the Success field of a Dependency or Request. Success is `true` if
the status is `OK`; otherwise `false`.

For `INTERNAL` Spans the Dependency Type is always `"InProc"` and Success is `true`.

The following of the Span's attributes map to special fields in Application Insights (the
mapping tries to follow the OpenTelemetry semantic conventions for [trace] and [resource]).

[trace]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions
[resource]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/resource/semantic_conventions

| OpenTelemetry attribute key                    | Application Insights field     |
| ---------------------------------------------- | ------------------------------ |
| `service.version`                              | Context: Application version   |
| `enduser.id`                                   | Context: Authenticated user id |
| `service.namespace` + `service.name`           | Context: Cloud role            |
| `service.instance.id`                          | Context: Cloud role instance   |
| `telemetry.sdk.name` + `telemetry.sdk.version` | Context: Internal SDK version  |
| `http.url`                                     | Dependency Data                |
| `db.statement`                                 | Dependency Data                |
| `http.host`                                    | Dependency Target              |
| `net.peer.name` + `net.peer.port`              | Dependency Target              |
| `net.peer.ip` + `net.peer.port`                | Dependency Target              |
| `db.name`                                      | Dependency Target              |
| `http.status_code`                             | Dependency Result code         |
| `db.system`                                    | Dependency Type                |
| `messaging.system`                             | Dependency Type                |
| `rpc.system`                                   | Dependency Type                |
| `"HTTP"` if any `http.` attribute exists       | Dependency Type                |
| `"DB"` if any `db.` attribute exists           | Dependency Type                |
| `http.url`                                     | Request Url                    |
| `http.scheme` + `http.host` + `http.target`    | Request Url                    |
| `http.client_ip`                               | Request Source                 |
| `net.peer.ip`                                  | Request Source                 |
| `http.status_code`                             | Request Response code          |

All other attributes are directly converted to custom properties.

For Requests the attributes `http.method` and `http.route` override the Name.

### Events

Events are converted into Exception telemetry if the event name equals `"exception"` (see
OpenTelemetry semantic conventions for [exceptions]) with the following mapping:

| OpenTelemetry attribute key | Application Insights field |
| --------------------------- | -------------------------- |
| `exception.type`            | Exception type             |
| `exception.message`         | Exception message          |
| `exception.stacktrace`      | Exception call stack       |

All other events are converted into Trace telemetry.

All other attributes are directly converted to custom properties.

[exceptions]: https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/semantic_conventions/exceptions.md

## Application Insights integration

### Thanks

Huge thanks goes to [Denis Molokanov] for the amazing [appinsights] crate.
Check it out if you want a more direct integration with Application Insights.

[Denis Molokanov]: https://github.com/dmolokanov
[appinsights]: https://github.com/dmolokanov/appinsights-rs

### Documentation

The only official documentation I could find is this one. Follow the links to
see the data model and endpoint description.

> Can I send telemetry to the Application Insights portal?
>
> We recommend you use our SDKs and use the [SDK API]. There are variants of
> the SDK for various [platforms]. These SDKs handle buffering, compression,
> throttling, retries, and so on. However, the [ingestion schema] and [endpoint
> protocol] are public.
>
> -- https://docs.microsoft.com/en-us/azure/azure-monitor/faq#can-i-send-telemetry-to-the-application-insights-portal

[SDK API]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/api-custom-events-metrics
[platforms]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/platforms
[ingestion schema]: https://github.com/microsoft/ApplicationInsights-dotnet/tree/master/BASE/Schema/PublicSchema
[endpoint protocol]: https://github.com/Microsoft/ApplicationInsights-Home/blob/master/EndpointSpecs/ENDPOINT-PROTOCOL.md
