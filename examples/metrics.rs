use opentelemetry::{metrics::Unit, KeyValue};
use rand::{thread_rng, Rng};
use std::{error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let exporter =
        opentelemetry_application_insights::new_exporter_from_env(reqwest::Client::new())
            .expect("valid connection string");
    let meter = opentelemetry_application_insights::new_pipeline(exporter)
        .metrics()
        .with_interval(Duration::from_secs(1))
        .install(opentelemetry_sdk::runtime::Tokio);

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
