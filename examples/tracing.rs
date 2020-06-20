use opentelemetry::{
    api::{HttpTextFormat, Provider, TraceContextPropagator},
    sdk,
};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::delay_for;
use tracing::Span;
use tracing_attributes::instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[instrument]
async fn spawn_children(n: u32, process_name: String) {
    for _ in 0..n {
        spawn_child_process(&process_name).await;
    }
}

#[instrument]
async fn spawn_child_process(process_name: &str) {
    let mut carrier = HashMap::new();
    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&Span::current().context(), &mut carrier);
    let child = Command::new(process_name)
        .arg(
            carrier
                .remove("traceparent".into())
                .expect("popagator should inject traceparent"),
        )
        .spawn();
    let future = child.expect("failed to spawn");
    future.await.expect("awaiting process failed");
}

#[instrument]
async fn run_in_child_process() {
    tracing::info!("leaf fn");
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
            service_name: "example-tracing".to_string(),
            tags: vec![],
        })
        .init()?;
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.get_tracer("example-tracing");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber).expect("setting global default failed");

    match traceparent {
        Some(traceparent) => {
            let mut carrier = HashMap::new();
            carrier.insert("traceparent".to_string(), traceparent);
            let propagator = TraceContextPropagator::new();
            let cx = propagator.extract(&carrier);
            let span = tracing::info_span!("child");
            span.set_parent(&cx);
            let _guard = span.enter();
            run_in_child_process().await;
        }
        _ => {
            let span = tracing::info_span!("root");
            let _guard = span.enter();
            spawn_children(5, process_name).await;
        }
    }

    Ok(())
}
