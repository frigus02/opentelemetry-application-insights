use opentelemetry::{
    global,
    metrics::Unit,
    sdk::{
        export::metrics::aggregation::stateless_temporality_selector,
        metrics::{controllers, processors, selectors},
    },
    Context, KeyValue,
};
use rand::{thread_rng, Rng};
use std::{env, error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key, ());
    let controller = controllers::basic(processors::factory(
        selectors::simple::histogram(vec![230., 260., 300.]),
        stateless_temporality_selector(), // TODO: make configurable in exporter
    ))
    .with_exporter(exporter)
    .with_collect_period(Duration::from_secs(1))
    .build();

    let cx = Context::new();
    controller.start(&cx, opentelemetry::runtime::Tokio)?;
    global::set_meter_provider(controller.clone());

    let meter = global::meter("custom.instrumentation");

    // Observable
    let cpu_utilization_gauge = meter
        .f64_observable_gauge("system.cpu.utilization")
        .with_unit(Unit::new("1"))
        .init();
    meter.register_callback(move |cx| {
        let mut rng = thread_rng();
        cpu_utilization_gauge.observe(
            cx,
            rng.gen_range(0.1..0.2),
            &[KeyValue::new("state", "idle"), KeyValue::new("cpu", 0)],
        )
    })?;

    // Recorder
    let server_duration = meter
        .u64_histogram("http.server.duration")
        .with_unit(Unit::new("milliseconds"))
        .init();
    let cx = Context::current();
    let mut rng = thread_rng();
    for _ in 1..5 {
        server_duration.record(
            &cx,
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

    controller.stop(&cx)?;

    Ok(())
}
