[![Crates.io {{version}}](https://img.shields.io/crates/v/opentelemetry-application-insights.svg)](https://crates.io/crates/opentelemetry-application-insights)
[![Documentation {{version}}](https://docs.rs/opentelemetry-application-insights/badge.svg)](https://docs.rs/opentelemetry-application-insights)
{{badges}}

# {{crate}}

{{readme}}

## Thanks

This is based on the amazing work by [Denis Molokanov](https://github.com/dmolokanov). Check out the [appinsights](https://github.com/dmolokanov/appinsights-rs) crate, if you want a more direct integration with Application Insights.

## Application Insights integration

The integration is based on resources mentioned here:

> Can I send telemetry to the Application Insights portal?
>
> We recommend you use our SDKs and use the [SDK API](https://docs.microsoft.com/en-us/azure/azure-monitor/app/api-custom-events-metrics). There are variants of the SDK for various [platforms](https://docs.microsoft.com/en-us/azure/azure-monitor/app/platforms). These SDKs handle buffering, compression, throttling, retries, and so on. However, the [ingestion schema](https://github.com/microsoft/ApplicationInsights-dotnet/tree/master/BASE/Schema/PublicSchema) and [endpoint protocol](https://github.com/Microsoft/ApplicationInsights-Home/blob/master/EndpointSpecs/ENDPOINT-PROTOCOL.md) are public.

https://docs.microsoft.com/en-us/azure/azure-monitor/faq#can-i-send-telemetry-to-the-application-insights-portal
