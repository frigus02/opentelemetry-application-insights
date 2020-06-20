# opentelemetry-application-insights

## Application Insights integration

The integration is based on resources mentioned here:

> Can I send telemetry to the Application Insights portal?
>
> We recommend you use our SDKs and use the [SDK API](https://docs.microsoft.com/en-us/azure/azure-monitor/app/api-custom-events-metrics). There are variants of the SDK for various [platforms](https://docs.microsoft.com/en-us/azure/azure-monitor/app/platforms). These SDKs handle buffering, compression, throttling, retries, and so on. However, the [ingestion schema](https://github.com/microsoft/ApplicationInsights-dotnet/tree/master/BASE/Schema/PublicSchema) and [endpoint protocol](https://github.com/Microsoft/ApplicationInsights-Home/blob/master/EndpointSpecs/ENDPOINT-PROTOCOL.md) are public.

https://docs.microsoft.com/en-us/azure/azure-monitor/faq#can-i-send-telemetry-to-the-application-insights-portal

The schema bond files (commit [7633ae849edc826a8547745b6bf9f3174715d4bd](https://github.com/microsoft/ApplicationInsights-dotnet/tree/7633ae849edc826a8547745b6bf9f3174715d4bd/BASE/Schema/PublicSchema) can be converted to JSON using the [Bond compiler](https://microsoft.github.io/bond/manual/compiler.html).

```sh
git clone https://github.com/microsoft/ApplicationInsights-dotnet temp-schema
cd temp-schema/BASE/Schema/PublicSchema/
gbc schema *.bond
```

The amazing [appinsights-contracts-codegen](https://github.com/dmolokanov/appinsights-rs/tree/6c535f3c70b84e980c5fe01f5f728dd94b4c2244/appinsights-contracts-codegen) uses those JSON files to generate Rust Structs. Unfortunatelty the code generation isn't 100% correct, so some files had to be modified manually afterwards.
