use opentelemetry::trace::Tracer as _;
use std::env;

#[tokio::main]
async fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let tracer = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(reqwest::Client::new())
        .install_batch(opentelemetry::runtime::Tokio);

    tracer.in_span("reqwest-client", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
