use opentelemetry::trace::Tracer as _;
use opentelemetry_sdk::metrics::exporter;

#[async_std::main]
async fn main() {
    env_logger::init();

    let exporter = opentelemetry_application_insights::new_exporter_from_env(
        isahc::HttpClient::new().unwrap(),
    )
    .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist");
    let tracer = opentelemetry_application_insights::new_pipeline(exporter)
        .traces()
        .install_batch(opentelemetry_sdk::runtime::AsyncStd);

    tracer.in_span("isahc-client", |_cx| {});

    opentelemetry::global::shutdown_tracer_provider();
}
