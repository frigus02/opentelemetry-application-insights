use opentelemetry::trace::{Tracer, TracerProvider};

fn main() {
    env_logger::init();

    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")
            .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist"),
        reqwest::blocking::Client::new(),
    )
    .expect("valid connection string");
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = tracer_provider.tracer("test");

    tracer.in_span("reqwest-blocking-client", |_cx| {});

    tracer_provider.shutdown().unwrap();
}
