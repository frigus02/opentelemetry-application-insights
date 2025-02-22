use opentelemetry::{trace::Tracer as _, trace::TracerProvider as _};

#[tokio::main]
async fn main() {
    env_logger::init();

    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")
            .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist"),
        reqwest::Client::new(),
    )
    .expect("valid connection string");
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_span_processor(opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor::builder(exporter, opentelemetry_sdk::runtime::Tokio).build())
        .build();
    let tracer = tracer_provider.tracer("test");

    tracer.in_span("reqwest-client", |_cx| {});

    tracer_provider.shutdown().unwrap();
}
