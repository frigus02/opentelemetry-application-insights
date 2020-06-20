# OpenTelemetry exporter for Azure Application Insights

An [Aure Application Insights](https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview) exporter implementation for [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust).

**Disclaimer**: This is not an official Microsoft product.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
opentelemetry-application-insights = "0.1"
```

## Usage

Configure the exporter:

```rust
use opentelemetry::{global, sdk};

fn init_tracer() {
    let instrumentation_key = "...";
    let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key);
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .with_config(sdk::Config {
            default_sampler: Box::new(sdk::Sampler::AlwaysOn),
            ..Default::default()
        })
        .build();
    global::set_provider(provider);
}
```

Then follow documentation of the[opentracing](https://github.com/open-telemetry/opentelemetry-rust) library to submit spans and events.

## Attribute mapping

The span kind determines the type of telemetry tracked in Application Insights:

| Span kind                  | Telemetry type |
| -------------------------- | -------------- |
| Client, Provider           | Dependency     |
| Server, Consumer, Internal | Request        |

The following attributes map to special fields in Application Insights:

| Attribute key            | Application Insights field                  |
| ------------------------ | ------------------------------------------- |
| span status              | request/dependency success (`true` if `OK`) |
| `request.source`         | request source                              |
| `request.response_code`  | request response code                       |
| `request.url`            | request url                                 |
| `dependency.result_code` | dependency result code                      |
| `dependency.data`        | dependency data                             |
| `dependency.target`      | dependency target                           |
| `dependency.type`        | dependency type                             |

All other attributes will be reported as custom properties.

**TODO:** Support semantic conventions: https://github.com/open-telemetry/opentelemetry-specification/tree/d46c3618552e70676c92e2a8d1552e77770c0cce/specification/trace/semantic_conventions

## Thanks

This is based on the amazing work of [Denis Molokanov](https://github.com/dmolokanov) with the [appinsights](https://github.com/dmolokanov/appinsights-rs) crate.

## Application Insights integration

The integration is based on resources mentioned here:

> Can I send telemetry to the Application Insights portal?
>
> We recommend you use our SDKs and use the [SDK API](https://docs.microsoft.com/en-us/azure/azure-monitor/app/api-custom-events-metrics). There are variants of the SDK for various [platforms](https://docs.microsoft.com/en-us/azure/azure-monitor/app/platforms). These SDKs handle buffering, compression, throttling, retries, and so on. However, the [ingestion schema](https://github.com/microsoft/ApplicationInsights-dotnet/tree/master/BASE/Schema/PublicSchema) and [endpoint protocol](https://github.com/Microsoft/ApplicationInsights-Home/blob/master/EndpointSpecs/ENDPOINT-PROTOCOL.md) are public.

https://docs.microsoft.com/en-us/azure/azure-monitor/faq#can-i-send-telemetry-to-the-application-insights-portal
