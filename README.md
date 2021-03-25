[![Crates.io 0.13.0](https://img.shields.io/crates/v/opentelemetry-application-insights.svg)](https://crates.io/crates/opentelemetry-application-insights)
[![Documentation 0.13.0](https://docs.rs/opentelemetry-application-insights/badge.svg)](https://docs.rs/opentelemetry-application-insights)
[![Workflow Status](https://github.com/frigus02/opentelemetry-application-insights/workflows/CI/badge.svg)](https://github.com/frigus02/opentelemetry-application-insights/actions?query=workflow%3A%22CI%22)

# opentelemetry-application-insights

An [Azure Application Insights] exporter implementation for [OpenTelemetry Rust].

[Azure Application Insights]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview
[OpenTelemetry Rust]: https://github.com/open-telemetry/opentelemetry-rust

**Disclaimer**: This is not an official Microsoft product.

## Usage

Configure a OpenTelemetry pipeline using the Application Insights exporter and start creating
spans (this example requires the **reqwest-client-blocking** feature):

```rust
use opentelemetry::trace::Tracer as _;

fn main() {
    let instrumentation_key = std::env::var("INSTRUMENTATION_KEY").unwrap();
    let tracer = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(reqwest::blocking::Client::new())
        .install_simple();

    tracer.in_span("main", |_cx| {});
}
```

### Simple or Batch

The functions `build_simple` and `install_simple` build/install a trace pipeline using the
simple span processor. This means each span is processed and exported synchronously at the time
it ends.

The functions `build_batch` and `install_batch` use the batch span processor instead. This
means spans are exported periodically in batches, which can be better for performance. This
feature requires an async runtime such as Tokio or async-std. If you decide to use a batch span
processor, make sure to call `opentelemetry::global::shutdown_tracer_provider()` before your
program exits to ensure all remaining spans are exported properly (this example requires the
**reqwest-client** and **opentelemetry/rt-tokio** features).

```rust
use opentelemetry::trace::Tracer as _;

#[tokio::main]
async fn main() {
    let instrumentation_key = std::env::var("INSTRUMENTATION_KEY").unwrap();
    let tracer = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(reqwest::Client::new())
        .install_batch(opentelemetry::runtime::Tokio);

    tracer.in_span("main", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
```

### Features

In order to support different async runtimes, the exporter requires you to specify an HTTP
client that works with your chosen runtime. This crate comes with support for:

- [`surf`] for [`async-std`]: enable the **surf-client** and **opentelemetry/rt-async-std**
  features and configure the exporter with `with_client(surf::Client::new())`.
- [`reqwest`] for [`tokio`]: enable the **reqwest-client** and **opentelemetry/rt-tokio** features
  and configure the exporter with `with_client(reqwest::Client::new())`.
- [`reqwest`] for synchronous exports: enable the **reqwest-blocking-client** feature and
  configure the exporter with `with_client(reqwest::blocking::Client::new())`.

[`async-std`]: https://crates.io/crates/async-std
[`reqwest`]: https://crates.io/crates/reqwest
[`surf`]: https://crates.io/crates/surf
[`tokio`]: https://crates.io/crates/tokio

Alternatively you can bring any other HTTP client by implementing the `HttpClient` trait.

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

The Span's status determines the Success field of a Dependency or Request. Success is `false` if
the status `Error`; otherwise `true`.

The following of the Span's attributes map to special fields in Application Insights (the
mapping tries to follow the OpenTelemetry semantic conventions for [trace] and [resource]).

Note: for `INTERNAL` Spans the Dependency Type is always `"InProc"`.

[trace]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions
[resource]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/resource/semantic_conventions

| OpenTelemetry attribute key                       | Application Insights field                               |
| ------------------------------------------------- | -----------------------------------------------------    |
| `service.version`                                 | Context: Application version (`ai.application.ver`)      |
| `enduser.id`                                      | Context: Authenticated user id (`ai.user.authUserId`)    |
| `service.namespace` + `service.name`              | Context: Cloud role (`ai.cloud.role`)                    |
| `service.instance.id`                             | Context: Cloud role instance (`ai.cloud.roleInstance`)   |
| `telemetry.sdk.name` + `telemetry.sdk.version`    | Context: Internal SDK version (`ai.internal.sdkVersion`) |
| `SpanKind::Server` + `http.method` + `http.route` | Context: Operation Name (`ai.operation.name`)            |
| `ai.*`                                            | Context: AppInsights Tag (`ai.*`)                        |
| `http.url`                                        | Dependency Data                                          |
| `db.statement`                                    | Dependency Data                                          |
| `http.host`                                       | Dependency Target                                        |
| `net.peer.name` + `net.peer.port`                 | Dependency Target                                        |
| `net.peer.ip` + `net.peer.port`                   | Dependency Target                                        |
| `db.name`                                         | Dependency Target                                        |
| `http.status_code`                                | Dependency Result code                                   |
| `db.system`                                       | Dependency Type                                          |
| `messaging.system`                                | Dependency Type                                          |
| `rpc.system`                                      | Dependency Type                                          |
| `"HTTP"` if any `http.` attribute exists          | Dependency Type                                          |
| `"DB"` if any `db.` attribute exists              | Dependency Type                                          |
| `http.url`                                        | Request Url                                              |
| `http.scheme` + `http.host` + `http.target`       | Request Url                                              |
| `http.client_ip`                                  | Request Source                                           |
| `net.peer.ip`                                     | Request Source                                           |
| `http.status_code`                                | Request Response code                                    |

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
