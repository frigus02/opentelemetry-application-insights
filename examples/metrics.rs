use opentelemetry::{
    global,
    metrics::{ObserverResult, Unit},
    sdk::{self, metrics::controllers},
    KeyValue,
};
use rand::{thread_rng, Rng};
use std::{env, time::Duration};

#[tokio::main]
async fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key, ());
    let controller = controllers::push(
        sdk::metrics::selectors::simple::Selector::Exact,
        sdk::export::metrics::ExportKindSelector::Stateless,
        exporter,
        tokio::spawn,
        opentelemetry::util::tokio_interval_stream,
    )
    .with_period(Duration::from_secs(1))
    .build();

    global::set_meter_provider(controller.provider());

    let meter = global::meter("custom.instrumentation");

    // Observer
    let cpu_utilization_callback = |res: ObserverResult<f64>| {
        let mut rng = thread_rng();
        res.observe(
            rng.gen_range(0.1..0.2),
            &[KeyValue::new("state", "idle"), KeyValue::new("cpu", 0)],
        )
    };
    let _ = meter
        .f64_value_observer("system.cpu.utilization", cpu_utilization_callback)
        .with_unit(Unit::new("1"))
        .init();

    // Recorder
    let value_recorder = meter
        .i64_value_recorder("http.server.duration")
        .with_unit(Unit::new("milliseconds"))
        .init();
    let mut rng = thread_rng();
    for _ in 1..5 {
        value_recorder.record(
            rng.gen_range(200..300),
            &[
                KeyValue::new("http.method", "GET"),
                KeyValue::new("http.host", "10.1.2.4"),
                KeyValue::new("http.scheme", "http"),
                KeyValue::new("http.target", "/hello/world?name={}"),
                KeyValue::new("http.status_code", "200"),
            ],
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tokio::time::sleep(Duration::from_secs(3)).await;
}
