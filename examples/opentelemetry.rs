use opentelemetry::{
    api::{
        trace::futures::FutureExt, Context, HttpTextFormat, Key, Span, SpanKind, TraceContextExt,
        TraceContextPropagator, Tracer,
    },
    global, sdk,
};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::delay_for;

async fn spawn_children(n: u32, process_name: String) {
    let tracer = global::tracer("spawn_children");
    let span = tracer.start("spawn_children");
    span.set_attribute(Key::new("n").u64(n.into()));
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
    let span = tracer
        .span_builder("spawn_child_process")
        .with_kind(SpanKind::Client)
        .start(&tracer);
    span.set_attribute(Key::new("process_name").string(process_name.to_string()));
    let cx = Context::current_with_span(span);

    let mut injector = HashMap::new();
    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&cx, &mut injector);
    let child = Command::new(process_name)
        .arg(
            injector
                .remove("traceparent")
                .expect("propagator should inject traceparent"),
        )
        .spawn();
    let future = child.expect("failed to spawn");
    future
        .with_context(cx)
        .await
        .expect("awaiting process failed");
}

async fn run_in_child_process() {
    let tracer = global::tracer("run_in_child_process");
    let span = tracer.start("run_in_child_process");
    span.add_event("leaf fn".into(), vec![]);
    delay_for(Duration::from_millis(50)).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut iter = env::args();
    let process_name = iter.next().expect("0th argument should exist");
    let traceparent = iter.next();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");
    let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key);
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .build();
    global::set_provider(provider);

    match traceparent {
        Some(traceparent) => {
            let mut extractor = HashMap::new();
            extractor.insert("traceparent".to_string(), traceparent);
            let propagator = TraceContextPropagator::new();
            let _guard = propagator.extract(&extractor).attach();
            let tracer = global::tracer("example-opentelemetry");
            let span = tracer
                .span_builder("child")
                .with_kind(SpanKind::Server)
                .start(&tracer);
            let cx = Context::current_with_span(span);
            run_in_child_process().with_context(cx).await;
        }
        _ => {
            let tracer = global::tracer("example-opentelemetry");
            let span = tracer.start("root");
            let cx = Context::current_with_span(span);
            spawn_children(5, process_name).with_context(cx).await;
        }
    }

    Ok(())
}
