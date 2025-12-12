use opentelemetry::{propagation::TextMapPropagator, trace::TracerProvider as _};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::Span;
use tracing_attributes::instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[instrument]
async fn spawn_children(n: u32, process_name: String) {
    for child_no in 0..n {
        spawn_child_process(&process_name, child_no).await;
    }
}

#[instrument(fields(otel.kind = "client"))]
async fn spawn_child_process(process_name: &str, child_no: u32) {
    let mut injector = HashMap::new();
    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&Span::current().context(), &mut injector);
    let mut child = Command::new(process_name)
        .arg(
            injector
                .remove("traceparent")
                .expect("propagator should inject traceparent"),
        )
        .arg(child_no.to_string())
        .spawn()
        .expect("failed to spawn");
    child.wait().await.expect("awaiting process failed");
}

#[instrument]
async fn run_in_child_process(child_no: u32) {
    tracing::info!("leaf fn");
    sleep(Duration::from_millis(50)).await;
    if (child_no + 1) % 4 == 0 {
        let error: Box<dyn std::error::Error> = "An error".into();
        tracing::error!(error = error, "exception");
        // or: tracing::error!(error = "An error");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut iter = env::args();
    let process_name = iter.next().expect("0th argument should exist");
    let traceparent = iter.next();
    let child_no = iter.next();

    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        reqwest::Client::new(),
    )
    .expect("valid connection string");
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();
    let telemetry = tracing_opentelemetry::layer().with_tracer(provider.tracer("tracing-example"));
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber).expect("setting global default failed");

    match traceparent {
        Some(traceparent) => {
            let mut extractor = HashMap::new();
            extractor.insert("traceparent".to_string(), traceparent);
            let propagator = TraceContextPropagator::new();
            let cx = propagator.extract(&extractor);
            let span = tracing::info_span!("child", otel.kind = "server");
            span.set_parent(cx)?;
            let _guard = span.enter();
            run_in_child_process(
                child_no
                    .expect("child process has child_no arg")
                    .parse()
                    .expect("child_no arg is u32"),
            )
            .await;
        }
        _ => {
            let span = tracing::info_span!("root");
            let _guard = span.enter();
            spawn_children(5, process_name).await;
        }
    }

    provider.shutdown()?;

    Ok(())
}
