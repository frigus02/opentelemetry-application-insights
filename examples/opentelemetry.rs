use opentelemetry::{
    global,
    propagation::TextMapPropagator,
    trace::{FutureExt, Span, SpanKind, TraceContextExt, Tracer, TracerProvider},
    Context, KeyValue,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

async fn spawn_children(n: u32, process_name: String) {
    let tracer = global::tracer("spawn_children");
    let mut span = tracer.start("spawn_children");
    span.set_attributes([
        KeyValue::new("n", Into::<i64>::into(n)),
        KeyValue::new("process_name", process_name.clone()),
    ]);
    let cx = Context::current_with_span(span);
    for child_no in 0..n {
        spawn_child_process(&process_name, child_no)
            .with_context(cx.clone())
            .await;
    }
}

async fn spawn_child_process(process_name: &str, child_no: u32) {
    let tracer = global::tracer("spawn_child_process");
    let mut span = tracer
        .span_builder("spawn_child_process")
        .with_kind(SpanKind::Client)
        .start(&tracer);
    span.set_attribute(KeyValue::new("process_name", process_name.to_string()));
    span.set_attribute(KeyValue::new("child_no", child_no as i64));
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
        .arg(child_no.to_string())
        .spawn()
        .expect("failed to spawn");
    child
        .wait()
        .with_context(cx)
        .await
        .expect("awaiting process failed");
}

async fn run_in_child_process(child_no: u32) {
    let tracer = global::tracer("run_in_child_process");
    let mut span = tracer.start("run_in_child_process");
    span.set_attribute(KeyValue::new("child_no", child_no as i64));
    span.add_event("leaf fn", vec![]);
    sleep(Duration::from_millis(50)).await;
    if (child_no + 1) % 4 == 0 {
        let error: Box<dyn std::error::Error> = "An error".into();
        span.record_error(error.as_ref());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut iter = env::args();
    let process_name = iter.next().expect("0th argument should exist");
    let traceparent = iter.next();
    let child_no = iter.next();

    let tracer_provider = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::Client::new())
        .build_batch(opentelemetry_sdk::runtime::Tokio);
    let tracer = tracer_provider.tracer("test");

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
            run_in_child_process(
                child_no
                    .expect("child process has child_no arg")
                    .parse()
                    .expect("child_no arg is u32"),
            )
            .with_context(cx)
            .await;
        }
        _ => {
            let span = tracer.start("root");
            let cx = Context::current_with_span(span);
            spawn_children(5, process_name).with_context(cx).await;
        }
    }

    tracer_provider.shutdown()?;

    Ok(())
}
