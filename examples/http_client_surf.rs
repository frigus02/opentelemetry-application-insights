use opentelemetry::trace::Tracer as _;
use std::env;

#[async_std::main]
async fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let tracer = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(surf::Client::new())
        .install_batch(opentelemetry::runtime::AsyncStd);

    tracer.in_span("surf-client-1", |_cx| {});
    tracer.in_span("surf-client-2", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
