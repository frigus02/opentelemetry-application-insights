use opentelemetry::{
    global,
    propagation::TextMapPropagator,
    sdk::propagation::TraceContextPropagator,
    trace::{FutureExt, Span, SpanKind, TraceContextExt, Tracer},
    Context, Key,
};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

async fn spawn_children(n: u32, process_name: String) {
    let tracer = global::tracer("spawn_children");
    let mut span = tracer.start("spawn_children");
    span.set_attribute(Key::new("n").i64(n.into()));
    span.set_attribute(Key::new("process_name").string(process_name.clone()));
    let cx = Context::current_with_span(span);
    for _ in 0..n {
        spawn_child_process(&process_name)
            .with_context(cx.clone())
            .await;
    }
}

async fn spawn_child_process(process_name: &str) {
    let tracer = global::tracer("spawn_child_process");
    let mut span = tracer
        .span_builder("spawn_child_process")
        .with_kind(SpanKind::Client)
        .start(&tracer);
    span.set_attribute(Key::new("process_name").string(process_name.to_string()));
    let cx = Context::current_with_span(span);

    let mut injector = HashMap::new();
    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&cx, &mut injector);
    let mut child = Command::new(process_name)
        .arg(
            injector
                .remove("traceparent")
                .expect("propagator should inject traceparent"),
        )
        .spawn()
        .expect("failed to spawn");
    child
        .wait()
        .with_context(cx)
        .await
        .expect("awaiting process failed");
}

async fn run_in_child_process() {
    let tracer = global::tracer("run_in_child_process");
    let mut span = tracer.start("run_in_child_process");
    span.add_event("leaf fn".into(), vec![]);
    sleep(Duration::from_millis(50)).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut iter = env::args();
    let process_name = iter.next().expect("0th argument should exist");
    let traceparent = iter.next();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");
    let tracer = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(reqwest::Client::new())
        .install_batch(opentelemetry::runtime::Tokio);

    match traceparent {
        Some(traceparent) => {
            let mut extractor = HashMap::new();
            extractor.insert("traceparent".to_string(), traceparent);
            let propagator = TraceContextPropagator::new();
            let _guard = propagator.extract(&extractor).attach();
            let span = tracer
                .span_builder("child")
                .with_kind(SpanKind::Server)
                .start(&tracer);
            let cx = Context::current_with_span(span);
            run_in_child_process().with_context(cx).await;
        }
        _ => {
            let span = tracer.start("root");
            let cx = Context::current_with_span(span);
            spawn_children(5, process_name).with_context(cx).await;
        }
    }

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
