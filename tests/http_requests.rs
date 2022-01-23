use async_trait::async_trait;
use bytes::Bytes;
use http::{Request, Response};
use opentelemetry::{
    sdk::{trace::Config, Resource},
    trace::{get_active_span, SpanKind, Tracer, TracerProvider},
    KeyValue,
};
use opentelemetry_application_insights::new_pipeline;
use opentelemetry_http::{HttpClient, HttpError};
use opentelemetry_semantic_conventions as semcov;
use regex::Regex;
use std::sync::{Arc, Mutex};

#[test]
fn http_requests() {
    let requests = create_traces()
        .into_iter()
        .map(request_to_string)
        .collect::<Vec<_>>()
        .join("\n\n\n");
    insta::assert_snapshot!(requests);
}

#[derive(Debug, Default, Clone)]
struct RecordingClient {
    requests: Arc<Mutex<Vec<Request<Vec<u8>>>>>,
}

impl RecordingClient {
    fn read_and_clear(&self) -> Vec<Request<Vec<u8>>> {
        self.requests
            .lock()
            .expect("requests mutex is healthy")
            .split_off(0)
    }
}

#[async_trait]
impl HttpClient for RecordingClient {
    async fn send(&self, req: Request<Vec<u8>>) -> Result<Response<Bytes>, HttpError> {
        self.requests
            .lock()
            .expect("requests mutex is healthy")
            .push(req);
        Ok(Response::builder()
            .status(200)
            .body(Bytes::from("{}"))
            .expect("response is fell formed"))
    }
}

fn create_traces() -> Vec<Request<Vec<u8>>> {
    let client = RecordingClient::default();

    {
        // Fake instrumentation key (this is a random uuid)
        let instrumentation_key = "0fdcec70-0ce5-4085-89d9-9ae8ead9af66".to_string();

        let client_provider = new_pipeline(instrumentation_key.clone())
            .with_client(client.clone())
            .with_trace_config(Config::default().with_resource(Resource::new(vec![
                semcov::resource::SERVICE_NAMESPACE.string("test"),
                semcov::resource::SERVICE_NAME.string("client"),
            ])))
            .build_simple();
        let client_tracer = client_provider.tracer("test");

        let server_provider = new_pipeline(instrumentation_key)
            .with_client(client.clone())
            .with_trace_config(Config::default().with_resource(Resource::new(vec![
                semcov::resource::SERVICE_NAMESPACE.string("test"),
                semcov::resource::SERVICE_NAME.string("server"),
            ])))
            .build_simple();
        let server_tracer = server_provider.tracer("test");

        // An HTTP client make a request
        let span = client_tracer
            .span_builder("dependency")
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                semcov::trace::ENDUSER_ID.string("marry"),
                semcov::trace::NET_HOST_NAME.string("localhost"),
                semcov::trace::NET_PEER_IP.string("10.1.2.4"),
                semcov::trace::HTTP_URL.string("http://10.1.2.4/hello/world?name=marry"),
                semcov::trace::HTTP_STATUS_CODE.string("200"),
            ])
            .start(&client_tracer);
        client_tracer.with_span(span, |cx| {
            // The server receives the request
            let builder = server_tracer
                .span_builder("request")
                .with_kind(SpanKind::Server)
                .with_attributes(vec![
                    semcov::trace::ENDUSER_ID.string("marry"),
                    semcov::trace::NET_HOST_NAME.string("localhost"),
                    semcov::trace::NET_PEER_IP.string("10.1.2.3"),
                    semcov::trace::HTTP_TARGET.string("/hello/world?name=marry"),
                    semcov::trace::HTTP_STATUS_CODE.string("200"),
                ]);
            let span = server_tracer.build_with_context(builder, &cx);
            server_tracer.with_span(span, |_cx| {
                get_active_span(|span| {
                    span.add_event("An event!", vec![KeyValue::new("happened", true)]);
                    let error: Box<dyn std::error::Error> = "An error".into();
                    span.record_exception_with_stacktrace(error.as_ref(), "a backtrace");
                });
            });
        });
    }

    client.read_and_clear()
}

fn request_to_string(req: Request<Vec<u8>>) -> String {
    let method = req.method();
    let path = req.uri().path_and_query().expect("path exists");
    let version = format!("{:?}", req.version());
    let host = req.uri().authority().expect("authority exists");
    let headers = req
        .headers()
        .into_iter()
        .map(|(name, value)| {
            let value = value.to_str().expect("header value is valid string");
            format!("{name}: {value}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = strip_changing_values(&pretty_print_json(req.body()));
    format!("{method} {path} {version}\nhost: {host}\n{headers}\n\n{body}")
}

fn strip_changing_values(body: &str) -> String {
    let res = vec![
        Regex::new(r#""(?P<field>time)": "\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}Z""#).unwrap(),
        Regex::new(r#""(?P<field>duration)": "\d+\.\d{2}:\d{2}:\d{2}\.\d{6}""#).unwrap(),
        Regex::new(r#""(?P<field>id|ai\.operation\.parentId)": "[a-z0-9]{16}""#).unwrap(),
        Regex::new(r#""(?P<field>ai\.operation\.id)": "[a-z0-9]{32}""#).unwrap(),
    ];

    res.into_iter().fold(body.into(), |body, re| {
        re.replace_all(&body, r#""$field": "STRIPPED""#).into()
    })
}

fn pretty_print_json(body: &[u8]) -> String {
    let json: serde_json::Value = serde_json::from_slice(body).expect("body is valid json");
    serde_json::to_string_pretty(&json).unwrap()
}
