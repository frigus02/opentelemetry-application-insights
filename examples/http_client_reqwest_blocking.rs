use opentelemetry::{trace::Tracer as _, trace::TracerProvider as _};

fn main() {
    env_logger::init();

    let tracer_provider = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::blocking::Client::new())
        .build_simple();
    let tracer = tracer_provider.tracer("test");

    tracer.in_span("reqwest-blocking-client", |_cx| {});

    tracer_provider.shutdown().unwrap();
}
