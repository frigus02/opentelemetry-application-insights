use opentelemetry::{trace::Tracer as _, trace::TracerProvider as _};

#[tokio::main]
async fn main() {
    env_logger::init();

    let tracer_provider = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::Client::new())
        .build_batch(opentelemetry_sdk::runtime::Tokio);
    let tracer = tracer_provider.tracer("test");

    tracer.in_span("reqwest-client", |_cx| {});

    tracer_provider.shutdown().unwrap();
}
