use opentelemetry::{
    trace::{Span, SpanKind, Status, Tracer as _, TracerProvider as _},
    KeyValue,
};
use opentelemetry_semantic_conventions as semcov;
use rand::{thread_rng, Rng};
use std::{error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    env_logger::init();

    let exporter =
        opentelemetry_application_insights::Exporter::new_from_env(reqwest::Client::new())
            .expect("valid connection string");
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_span_processor(opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor::builder(exporter.clone(), opentelemetry_sdk::runtime::Tokio).build())
        .with_span_processor(opentelemetry_application_insights::LiveMetricsSpanProcessor::new(exporter, opentelemetry_sdk::runtime::Tokio))
        .build();
    let tracer = tracer_provider.tracer("test");

    println!("Simulating requests. Press Ctrl+C to stop.");

    let mut rng = thread_rng();
    loop {
        let success = rng.gen_ratio(9, 10);
        let _request = tracer
            .span_builder("request")
            .with_kind(SpanKind::Server)
            .with_status(if success {
                Status::Ok
            } else {
                Status::error("")
            })
            .with_attributes(vec![
                KeyValue::new(semcov::trace::HTTP_REQUEST_METHOD, "GET"),
                KeyValue::new(semcov::trace::URL_SCHEME, "https"),
                KeyValue::new(semcov::trace::URL_PATH, "/hello/world"),
                KeyValue::new(semcov::trace::URL_QUERY, "name=marry"),
                KeyValue::new(semcov::trace::HTTP_RESPONSE_STATUS_CODE, 200),
            ])
            .start(&tracer);
        {
            let mut db = tracer
                .span_builder("db")
                .with_kind(SpanKind::Client)
                .with_status(if success {
                    Status::Ok
                } else {
                    Status::error("")
                })
                .start(&tracer);
            if !success {
                let err: Box<dyn std::error::Error> = "An error".into();
                db.record_error(err.as_ref());
            }

            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
