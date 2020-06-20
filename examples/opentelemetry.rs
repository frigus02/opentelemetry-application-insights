use opentelemetry::{
    api::{
        trace::futures::FutureExt, Context, HttpTextFormat, Span, SpanKind, TraceContextExt,
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
    let span = tracer.start("spawn loop");
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
        .span_builder("spawning")
        .with_kind(SpanKind::Server)
        .start(&tracer);
    let cx = Context::current_with_span(span);

    let mut carrier = HashMap::new();
    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&cx, &mut carrier);
    let child = Command::new(process_name)
        .arg(
            carrier
                .remove("traceparent".into())
                .expect("popagator should inject traceparent"),
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
    let span = tracer
        .span_builder("running")
        .with_kind(SpanKind::Client)
        .start(&tracer);
    span.add_event("leaf fn".into(), vec![]);
    delay_for(Duration::from_millis(50)).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut iter = env::args();
    let process_name = iter.next().expect("0th argument should exist");
    let traceparent = iter.next();

    //let instrumentation_key =
    //    env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");
    //let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key);
    let exporter = opentelemetry_jaeger::Exporter::builder()
        .with_agent_endpoint("127.0.0.1:6831".parse().unwrap())
        .with_process(opentelemetry_jaeger::Process {
            service_name: "trace-demo".to_string(),
            tags: vec![],
        })
        .init()?;
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .build();
    global::set_provider(provider);

    match traceparent {
        Some(traceparent) => {
            let mut carrier = HashMap::new();
            carrier.insert("traceparent".to_string(), traceparent);
            let propagator = TraceContextPropagator::new();
            let cx = propagator.extract(&carrier);
            run_in_child_process().with_context(cx).await;
        }
        _ => {
            let tracer = global::tracer("opentelemetry_example");
            let span = tracer.start("root");
            let cx = Context::current_with_span(span);
            spawn_children(5, process_name).with_context(cx).await;
        }
    }

    Ok(())
}
