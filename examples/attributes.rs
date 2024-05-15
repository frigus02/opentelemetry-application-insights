use opentelemetry::{
    trace::{
        get_active_span, mark_span_as_active, Link, SpanKind, TraceContextExt, Tracer,
        TracerProvider,
    },
    Context, KeyValue,
};
use opentelemetry_application_insights::attrs as ai;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions as semcov;

fn log() {
    get_active_span(|span| {
        span.add_event("An event!", vec![KeyValue::new("happened", true)]);
    })
}

fn custom() {
    get_active_span(|span| {
        span.add_event(
            "ai.custom",
            vec![
                KeyValue::new(ai::CUSTOM_EVENT_NAME, "A custom event!"),
                KeyValue::new("some.data", 5),
            ],
        );
    })
}

fn exception() {
    get_active_span(|span| {
        let error: Box<dyn std::error::Error> = "An error".into();
        span.record_error(error.as_ref());
    })
}

fn main() {
    env_logger::init();

    let client_provider = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::blocking::Client::new())
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "example-attributes"),
                KeyValue::new(semcov::resource::SERVICE_NAME, "client"),
                KeyValue::new(semcov::resource::DEVICE_ID, "123"),
                KeyValue::new(semcov::resource::DEVICE_MODEL_NAME, "Foo Phone"),
            ])),
        )
        .build_simple();
    let client_tracer = client_provider.tracer("example-attributes");

    let server_provider = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::blocking::Client::new())
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "example-attributes"),
                KeyValue::new(semcov::resource::SERVICE_NAME, "server"),
            ])),
        )
        .build_simple();
    let server_tracer = server_provider.tracer("example-attributes");

    // An HTTP client make a request
    let span = client_tracer
        .span_builder("dependency")
        .with_kind(SpanKind::Client)
        .with_attributes(vec![
            KeyValue::new(semcov::trace::HTTP_REQUEST_METHOD, "GET"),
            KeyValue::new(semcov::trace::NETWORK_PROTOCOL_NAME, "http"),
            KeyValue::new(semcov::trace::NETWORK_PROTOCOL_VERSION, "1.1"),
            KeyValue::new(
                semcov::trace::URL_FULL,
                "https://example.com:8080/hello/world?name=marry",
            ),
            KeyValue::new(semcov::trace::SERVER_ADDRESS, "example.com"),
            KeyValue::new(semcov::trace::SERVER_PORT, 8080),
            KeyValue::new(semcov::trace::SERVER_SOCKET_ADDRESS, "10.1.2.4"),
            KeyValue::new(semcov::trace::HTTP_RESPONSE_STATUS_CODE, 200),
            KeyValue::new(semcov::trace::ENDUSER_ID, "marry"),
        ])
        .start(&client_tracer);
    {
        let cx = Context::current_with_span(span);
        let _client_guard = cx.clone().attach();
        // The server receives the request
        let builder = server_tracer
            .span_builder("request")
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new(semcov::trace::HTTP_REQUEST_METHOD, "GET"),
                KeyValue::new(semcov::trace::NETWORK_PROTOCOL_NAME, "http"),
                KeyValue::new(semcov::trace::NETWORK_PROTOCOL_VERSION, "1.1"),
                KeyValue::new(semcov::trace::URL_PATH, "/hello/world"),
                KeyValue::new(semcov::trace::URL_QUERY, "name=marry"),
                KeyValue::new(semcov::trace::SERVER_ADDRESS, "example.com"),
                KeyValue::new(semcov::trace::SERVER_PORT, 8080),
                KeyValue::new(semcov::trace::URL_SCHEME, "https"),
                KeyValue::new(semcov::trace::HTTP_ROUTE, "/hello/world"),
                KeyValue::new(semcov::trace::HTTP_RESPONSE_STATUS_CODE, 200),
                KeyValue::new(semcov::trace::CLIENT_ADDRESS, "10.1.2.3"),
                KeyValue::new(semcov::trace::CLIENT_SOCKET_ADDRESS, "10.1.2.2"),
                KeyValue::new(semcov::trace::USER_AGENT_ORIGINAL, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0"),
                KeyValue::new(semcov::trace::ENDUSER_ID, "marry"),
            ]);
        let span = server_tracer.build_with_context(builder, &cx);
        {
            let _server_guard = mark_span_as_active(span);
            log();
            custom();
            exception();

            get_active_span(|span| {
                let async_op_builder = server_tracer
                    .span_builder("async operation")
                    .with_links(vec![Link::new(span.span_context().clone(), Vec::new())]);
                let async_op_context = Context::new();
                let _span = server_tracer.build_with_context(async_op_builder, &async_op_context);
            })
        }
    }
}
