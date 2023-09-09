use opentelemetry::{
    global,
    metrics::Unit,
    sdk::{
        metrics::{MeterProvider, PeriodicReader},
        trace::TracerProvider,
    },
    trace::{SpanKind, Status, Tracer as _, TracerProvider as _},
    KeyValue,
};
use opentelemetry_semantic_conventions as semcov;
use rand::{thread_rng, Rng};
use std::{error::Error, time::Duration};

fn exporter(
    connection_string: &str,
) -> opentelemetry_application_insights::Exporter<reqwest::Client> {
    opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        reqwest::Client::new(),
    )
    .expect("valid connection string")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let reader =
        PeriodicReader::builder(exporter(&connection_string), opentelemetry::runtime::Tokio)
            .with_interval(Duration::from_secs(1))
            .build();
    let meter_provider = MeterProvider::builder().with_reader(reader).build();
    global::set_meter_provider(meter_provider);

    // LIVE METRICS START
    let quick_pulse = opentelemetry_application_insights::QuickPulseManager::new(
        exporter(&connection_string),
        opentelemetry::runtime::Tokio,
    );
    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter(&connection_string), opentelemetry::runtime::Tokio)
        .with_span_processor(quick_pulse)
        .build();
    let tracer = tracer_provider.tracer("example-metrics");
    // LIVE METRICS END

    let meter = global::meter("custom.instrumentation");

    // Observable
    let _cpu_utilization_gauge = meter
        .f64_observable_gauge("system.cpu.utilization")
        .with_unit(Unit::new("1"))
        .with_callback(|instrument| {
            let mut rng = thread_rng();
            instrument.observe(
                rng.gen_range(0.1..0.2),
                &[KeyValue::new("state", "idle"), KeyValue::new("cpu", 0)],
            )
        })
        .init();

    // Recorder
    let server_duration = meter
        .u64_histogram("http.server.duration")
        .with_unit(Unit::new("milliseconds"))
        .init();
    let mut rng = thread_rng();
    loop {
        server_duration.record(
            rng.gen_range(50..300),
            &[
                KeyValue::new("http.method", "GET"),
                KeyValue::new("http.host", "10.1.2.4"),
                KeyValue::new("http.scheme", "http"),
                KeyValue::new("http.target", "/hello/world?name={}"),
                KeyValue::new("http.status_code", "200"),
            ],
        );

        let _span = tracer
            .span_builder("request")
            .with_kind(SpanKind::Server)
            .with_status(if rng.gen_ratio(2, 3) {
                Status::Ok
            } else {
                Status::error("")
            })
            .with_attributes(vec![
                semcov::trace::HTTP_REQUEST_METHOD.string("GET"),
                semcov::trace::URL_SCHEME.string("https"),
                semcov::trace::URL_PATH.string("/hello/world"),
                semcov::trace::URL_QUERY.string("name=marry"),
                semcov::trace::HTTP_RESPONSE_STATUS_CODE.i64(200),
            ])
            .start(&tracer);

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
