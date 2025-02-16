use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use rand::{thread_rng, Rng};
use std::{error::Error, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        reqwest::blocking::Client::new(),
    )
    .expect("valid connection string");
    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(1))
        .build();
    let meter_provider = SdkMeterProvider::builder().with_reader(reader).build();
    global::set_meter_provider(meter_provider.clone());

    let meter = global::meter("custom.instrumentation");

    // Observable
    let _cpu_utilization_gauge = meter
        .f64_observable_gauge("system.cpu.utilization")
        .with_unit("1")
        .with_callback(|instrument| {
            let mut rng = thread_rng();
            instrument.observe(
                rng.gen_range(0.1..0.2),
                &[KeyValue::new("state", "idle"), KeyValue::new("cpu", 0)],
            )
        })
        .build();

    // Recorder
    let server_duration = meter
        .u64_histogram("http.server.duration")
        .with_unit("milliseconds")
        .build();
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
        std::thread::sleep(Duration::from_millis(500));
    }

    meter_provider.shutdown()?;

    Ok(())
}
