use opentelemetry::{
    global,
    metrics::Unit,
    sdk::metrics::{MeterProvider, PeriodicReader},
    KeyValue,
};
use rand::{thread_rng, Rng};
use std::{env, error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let exporter = opentelemetry_application_insights::Exporter::new(
        instrumentation_key,
        reqwest::Client::new(),
    );
    let reader = PeriodicReader::builder(exporter, opentelemetry::runtime::Tokio)
        .with_interval(Duration::from_secs(1))
        .build();
    let meter_provider = MeterProvider::builder().with_reader(reader).build();
    global::set_meter_provider(meter_provider);

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
    for _ in 1..10 {
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
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}
