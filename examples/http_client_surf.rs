use opentelemetry::trace::Tracer as _;

#[async_std::main]
async fn main() {
    env_logger::init();

    let tracer = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(surf::Client::new())
        .install_batch(opentelemetry::runtime::AsyncStd);

    tracer.in_span("surf-client", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
