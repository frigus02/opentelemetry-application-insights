[![Crates.io](https://img.shields.io/crates/v/opentelemetry-application-insights.svg)](https://crates.io/crates/opentelemetry-application-insights)
[![Documentation](https://docs.rs/opentelemetry-application-insights/badge.svg)](https://docs.rs/opentelemetry-application-insights)
[![Workflow Status](https://github.com/frigus02/opentelemetry-application-insights/workflows/CI/badge.svg)](https://github.com/frigus02/opentelemetry-application-insights/actions?query=workflow%3A%22CI%22)

# opentelemetry-application-insights

An [Azure Application Insights](https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview) exporter implementation for [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust).

**Disclaimer**: This is not an official Microsoft product.

## Usage

Configure a OpenTelemetry pipeline using the Application Insights exporter and start creating spans (this example requires the **opentelemetry-http/reqwest** feature):

```rust,no_run
use opentelemetry::trace::Tracer as _;

fn main() {
    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let tracer = opentelemetry_application_insights::new_pipeline_from_connection_string(connection_string)
        .expect("valid connection string")
        .with_client(reqwest::blocking::Client::new())
        .install_simple();

    tracer.in_span("main", |_cx| {});
}
```

See documentation for more:

- [Logs](https://docs.rs/opentelemetry-application-insights/latest/opentelemetry_application_insights/#logs)
- [Metrics](https://docs.rs/opentelemetry-application-insights/latest/opentelemetry_application_insights/#metrics)
- [Simple or Batch](https://docs.rs/opentelemetry-application-insights/latest/opentelemetry_application_insights/#simple-or-batch)
- [Async runtimes and HTTP clients](https://docs.rs/opentelemetry-application-insights/latest/opentelemetry_application_insights/#async-runtimes-and-http-clients)

## Application Insights integration

### Thanks

Huge thanks goes to [Denis Molokanov](https://github.com/dmolokanov) for the amazing [appinsights](https://github.com/dmolokanov/appinsights-rs) crate. Check it out if you want a more direct integration with Application Insights.

### Documentation

The only official documentation I could find is this one. Follow the links to see the data model and endpoint description.

> Can I send telemetry to the Application Insights portal?
>
> We recommend you use our SDKs and use the [SDK API](https://docs.microsoft.com/en-us/azure/azure-monitor/app/api-custom-events-metrics). There are variants of the SDK for various [platforms](https://docs.microsoft.com/en-us/azure/azure-monitor/app/platforms). These SDKs handle buffering, compression, throttling, retries, and so on. However, the [ingestion schema](https://github.com/microsoft/ApplicationInsights-dotnet/tree/master/BASE/Schema/PublicSchema) and [endpoint protocol](https://github.com/Microsoft/ApplicationInsights-Home/blob/master/EndpointSpecs/ENDPOINT-PROTOCOL.md) are public.
>
> -- https://docs.microsoft.com/en-us/azure/azure-monitor/faq#can-i-send-telemetry-to-the-application-insights-portal
