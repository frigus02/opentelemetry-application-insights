use opentelemetry::{
    api::{KeyValue, SpanKind, Tracer},
    global, sdk,
};
use std::env;

fn log() {
    global::tracer("log").get_active_span(|span| {
        span.add_event(
            "An event!".to_string(),
            vec![KeyValue::new("happened", true)],
        );
    })
}

fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");
    let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key);
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .build();
    global::set_provider(provider);

    let tracer = global::tracer("example-attributes");

    let span = tracer
        .span_builder("request")
        .with_kind(SpanKind::Server)
        .with_attributes(vec![
            KeyValue::new("enduser.id", "marry"),
            KeyValue::new("net.host.name", "localhost"),
            KeyValue::new("net.peer.ip", "10.1.2.3"),
            KeyValue::new("http.target", "/hello/world?name=marry"),
            KeyValue::new("http.status_code", "200"),
        ])
        .start(&tracer);
    tracer.with_span(span, |_cx| {
        log();
    });

    let span = tracer
        .span_builder("dependency")
        .with_kind(SpanKind::Client)
        .with_attributes(vec![
            KeyValue::new("enduser.id", "marry"),
            KeyValue::new("net.host.name", "localhost"),
            KeyValue::new("net.peer.ip", "10.1.2.4"),
            KeyValue::new("http.url", "http://10.1.2.4/hello/world?name=marry"),
            KeyValue::new("http.status_code", "200"),
        ])
        .start(&tracer);
    tracer.with_span(span, |_cx| {
        log();
    });
}
