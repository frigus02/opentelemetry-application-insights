use opentelemetry::trace::Tracer as _;

#[tokio::main]
async fn main() {
    env_logger::init();

    let tracer = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::Client::new())
        .install_batch(opentelemetry::runtime::Tokio);

    tracer.in_span("reqwest-client", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
