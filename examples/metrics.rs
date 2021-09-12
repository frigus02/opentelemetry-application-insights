use opentelemetry::{
    baggage::BaggageExt,
    global,
    metrics::ObserverResult,
    sdk::{self, metrics::controllers},
    Context, Key, KeyValue,
};
use std::{env, time::Duration};

fn common_attributes() -> Vec<KeyValue> {
    vec![
        KeyValue::new("A", "1"),
        KeyValue::new("B", "2"),
        KeyValue::new("C", "3"),
    ]
}

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

    let meter = global::meter("ex.com/basic");

    let one_metric_callback = |res: ObserverResult<f64>| res.observe(1.0, &common_attributes());
    let _ = meter
        .f64_value_observer("ex.com.one", one_metric_callback)
        .with_description("A ValueObserver set to 1.0")
        .init();

    let value_recorder = meter.f64_value_recorder("ex.com.two").init();
    meter.record_batch_with_context(
        &Context::current_with_baggage(vec![Key::from("ex.com/another").string("xyz")]),
        &common_attributes(),
        vec![value_recorder.measurement(2.0)],
    );

    tokio::time::sleep(Duration::from_secs(5)).await;
}
