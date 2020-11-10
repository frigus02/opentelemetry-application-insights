use opentelemetry::trace::Tracer;
use std::env;

#[async_std::main]
async fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let (tracer, _uninstall) =
        opentelemetry_application_insights::new_pipeline(instrumentation_key)
            .with_client(surf::Client::new())
            .install();

    tracer.in_span("surf-client", |_cx| {});
}
