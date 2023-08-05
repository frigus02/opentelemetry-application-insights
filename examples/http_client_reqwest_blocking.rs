use opentelemetry::trace::Tracer as _;

fn main() {
    env_logger::init();

    let tracer = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::blocking::Client::new())
        .install_simple();

    tracer.in_span("reqwest-blocking-client", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
