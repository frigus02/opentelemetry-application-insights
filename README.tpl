[![Crates.io {{version}}](https://img.shields.io/crates/v/opentelemetry-application-insights.svg)](https://crates.io/crates/opentelemetry-application-insights)
[![Documentation {{version}}](https://docs.rs/opentelemetry-application-insights/badge.svg)](https://docs.rs/opentelemetry-application-insights)
{{badges}}

# {{crate}}

{{readme}}

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
