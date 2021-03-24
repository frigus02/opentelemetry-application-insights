use opentelemetry::trace::Tracer as _;
use std::env;

fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let tracer = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(reqwest::blocking::Client::new())
        .install_simple();

    tracer.in_span("reqwest-blocking-client", |_cx| {});
}
