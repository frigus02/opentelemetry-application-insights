use opentelemetry::{
    trace::{
        get_active_span, mark_span_as_active, SpanKind, TraceContextExt, Tracer, TracerProvider,
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
                ai::CUSTOM_EVENT_NAME.string("A custom event!"),
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
                semcov::resource::SERVICE_NAMESPACE.string("example-attributes"),
                semcov::resource::SERVICE_NAME.string("client"),
            ])),
        )
        .build_simple();
    let client_tracer = client_provider.tracer("example-attributes");

    let server_provider = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(reqwest::blocking::Client::new())
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                semcov::resource::SERVICE_NAMESPACE.string("example-attributes"),
                semcov::resource::SERVICE_NAME.string("server"),
            ])),
        )
        .build_simple();
    let server_tracer = server_provider.tracer("example-attributes");

    // An HTTP client make a request
    let span = client_tracer
        .span_builder("dependency")
        .with_kind(SpanKind::Client)
        .with_attributes(vec![
            semcov::trace::HTTP_REQUEST_METHOD.string("GET"),
            semcov::trace::NETWORK_PROTOCOL_NAME.string("http"),
            semcov::trace::NETWORK_PROTOCOL_VERSION.string("1.1"),
            semcov::trace::URL_FULL.string("https://example.com:8080/hello/world?name=marry"),
            semcov::trace::SERVER_ADDRESS.string("example.com"),
            semcov::trace::SERVER_PORT.i64(8080),
            semcov::trace::SERVER_SOCKET_ADDRESS.string("10.1.2.4"),
            semcov::trace::HTTP_RESPONSE_STATUS_CODE.i64(200),
            semcov::trace::ENDUSER_ID.string("marry"),
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
                semcov::trace::HTTP_REQUEST_METHOD.string("GET"),
                semcov::trace::NETWORK_PROTOCOL_NAME.string("http"),
                semcov::trace::NETWORK_PROTOCOL_VERSION.string("1.1"),
                semcov::trace::URL_PATH.string("/hello/world"),
                semcov::trace::URL_QUERY.string("name=marry"),
                semcov::trace::SERVER_ADDRESS.string("example.com"),
                semcov::trace::SERVER_PORT.i64(8080),
                semcov::trace::URL_SCHEME.string("https"),
                semcov::trace::HTTP_ROUTE.string("/hello/world"),
                semcov::trace::HTTP_RESPONSE_STATUS_CODE.i64(200),
                semcov::trace::CLIENT_ADDRESS.string("10.1.2.3"),
                semcov::trace::CLIENT_SOCKET_ADDRESS.string("10.1.2.2"),
                semcov::trace::USER_AGENT_ORIGINAL.string("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0"),
                semcov::trace::ENDUSER_ID.string("marry"),
            ]);
        let span = server_tracer.build_with_context(builder, &cx);
        {
            let _server_guard = mark_span_as_active(span);
            log();
            custom();
            exception();
        }
    }
}
