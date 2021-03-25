use backtrace::Backtrace;
use opentelemetry::{
    sdk,
    trace::{get_active_span, SpanKind, Tracer, TracerProvider},
    KeyValue,
};
use std::env;

fn log() {
    get_active_span(|span| {
        span.add_event(
            "An event!".to_string(),
            vec![KeyValue::new("happened", true)],
        );
    })
}

fn exception() {
    get_active_span(|span| {
        let error: Box<dyn std::error::Error> = "An error".into();
        span.record_exception_with_stacktrace(error.as_ref(), format!("{:?}", Backtrace::new()));
    })
}

fn main() {
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let client_provider =
        opentelemetry_application_insights::new_pipeline(instrumentation_key.clone())
            .with_client(reqwest::blocking::Client::new())
            .with_trace_config(
                sdk::trace::Config::default().with_resource(sdk::Resource::new(vec![
                    KeyValue::new("service.namespace", "example-attributes"),
                    KeyValue::new("service.name", "client"),
                ])),
            )
            .build_simple();
    let client_tracer = client_provider.get_tracer("example-attributes", None);

    let server_provider = opentelemetry_application_insights::new_pipeline(instrumentation_key)
        .with_client(reqwest::blocking::Client::new())
        .with_trace_config(
            sdk::trace::Config::default().with_resource(sdk::Resource::new(vec![
                KeyValue::new("service.namespace", "example-attributes"),
                KeyValue::new("service.name", "server"),
            ])),
        )
        .build_simple();
    let server_tracer = server_provider.get_tracer("example-attributes", None);

    // An HTTP client make a request
    let span = client_tracer
        .span_builder("dependency")
        .with_kind(SpanKind::Client)
        .with_attributes(vec![
            KeyValue::new("enduser.id", "marry"),
            KeyValue::new("net.host.name", "localhost"),
            KeyValue::new("net.peer.ip", "10.1.2.4"),
            KeyValue::new("http.url", "http://10.1.2.4/hello/world?name=marry"),
            KeyValue::new("http.status_code", "200"),
        ])
        .start(&client_tracer);
    client_tracer.with_span(span, |cx| {
        // The server receives the request
        let builder = server_tracer
            .span_builder("request")
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("enduser.id", "marry"),
                KeyValue::new("net.host.name", "localhost"),
                KeyValue::new("net.peer.ip", "10.1.2.3"),
                KeyValue::new("http.target", "/hello/world?name=marry"),
                KeyValue::new("http.status_code", "200"),
            ])
            .with_parent_context(cx);
        let span = server_tracer.build(builder);
        server_tracer.with_span(span, |_cx| {
            log();
            exception();
        });
    });
}
