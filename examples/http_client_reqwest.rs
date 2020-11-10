use opentelemetry::trace::Tracer;
use std::env;

#[tokio::main]
async fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let (tracer, _uninstall) =
        opentelemetry_application_insights::new_pipeline(instrumentation_key)
            .with_client(reqwest::Client::new())
            .install();

    tracer.in_span("reqwest-client", |_cx| {});
}
