use opentelemetry::{
    global,
    trace::{FutureExt, SpanKind, TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_sdk::Resource;
use std::env;
use std::error::Error;
use std::ops::Add;
use std::time::{Duration, Instant, SystemTime};

async fn mock_sql_call(n: u64, duration: u64) {
    let tracer = global::tracer("run_in_child_process_new");
    let now = SystemTime::now();
    let end_time = now.add(Duration::from_millis(duration));
    tracer
        .span_builder("test_db")
        .with_kind(SpanKind::Client)
        .with_attributes(vec![
            KeyValue::new("service.name", "test-database"),
            KeyValue::new("db.system", "SQL"),
            KeyValue::new(
                "db.statement",
                format!("SELECT * FROM test WHERE test_id = {}", n),
            ),
        ])
        .with_start_time(now)
        .with_end_time(end_time)
        .start(&tracer);
}

async fn mock_serve_http_request(n: u64) {
    let tracer = global::tracer("named tracer");
    let now = SystemTime::now();
    let duration = 10 + (n % 50);
    let end_time = now.add(Duration::from_millis(duration));
    let span = tracer
        .span_builder("localhost")
        .with_attributes(vec![
            KeyValue::new("http.status_code", 200),
            KeyValue::new("http.client_id", "127.0.0.1"),
            KeyValue::new("http.server_name", "localhost:80"),
            KeyValue::new("http.http_method", "GET"),
            KeyValue::new("http.target", format!("/test/{}", n)),
            KeyValue::new("http.flavor", "1.1"),
            KeyValue::new("net.peer.id", "127.0.0.1:42424"),
            KeyValue::new("http.route", "/test/:test_id"),
            KeyValue::new("http.host", "localhost:80"),
            KeyValue::new("service.name", "test-http-server"),
        ])
        .with_start_time(now)
        .with_end_time(end_time)
        .with_kind(SpanKind::Server)
        .start(&tracer);

    let cx = Context::new().with_span(span);
    mock_sql_call(n, duration - 5).with_context(cx).await;
}

// This example emulates the traces that a typical HTTP server with a SQL server dependency would generate.
// The amount of traces generated is controlled by the NUM_ROOT_SPANS environment variable.
// WARNING: Please notice at large NUM_ROOT_SPANS settings, this can incur real costs at your application insights resource - so be cautious!
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    env_logger::init();

    // Please note with large NUM_ROOT_SPANS settings the batch span processor might start falling behind
    // You can mitigate this by configuring the batch span processor using the standard SDK environment variables
    // for instance:
    //
    // export OTEL_BSP_MAX_QUEUE_SIZE=200000
    //
    // For further details please refer to: https://opentelemetry.io/docs/reference/specification/sdk-environment-variables/#batch-span-processor
    let num_root_spans = env::var("NUM_ROOT_SPANS")
        .expect("env var NUM_ROOT_SPANS should exist")
        .parse::<u64>()
        .expect("NUM_ROOT_SPANS could not be parsed");

    let timer = Instant::now();

    // Must create blocking client outside the tokio runtime. Batch exporter will spawn a new
    // thread for exporting spans, so client usages will also happen outside the tokio runtime.
    let client = std::thread::spawn(reqwest::blocking::Client::new)
        .join()
        .unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")?,
        client,
    )?;
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name("test").build())
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    for i in 1..num_root_spans + 1 {
        mock_serve_http_request(i).await;
        if i % 1000 == 0 {
            println!("Mocked {} root spans", i);
        }
    }

    tracer_provider.shutdown()?;

    let duration = timer.elapsed();

    println!(
        "Finished uploading {} root spans in: {:?}",
        num_root_spans, duration
    );

    Ok(())
}
