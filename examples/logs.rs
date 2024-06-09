use opentelemetry::{
    logs::{LogRecord, Logger, LoggerProvider, Severity},
    trace::Tracer,
    KeyValue,
};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions as semcov;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    let tracer = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING should exist")
        .with_client(client.clone())
        .install_batch(opentelemetry_sdk::runtime::Tokio);

    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        client,
    )
    .expect("connection string is valid");
    let logger_provider = opentelemetry_sdk::logs::LoggerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            opentelemetry_sdk::logs::config().with_resource(Resource::new(vec![
                KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "test"),
                KeyValue::new(semcov::resource::SERVICE_NAME, "client"),
            ])),
        )
        .build();
    let otel_log_appender =
        opentelemetry_appender_log::OpenTelemetryLogBridge::new(&logger_provider);
    log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
    log::set_max_level(log::Level::Info.to_level_filter());

    // Log via `log` crate.
    let fruit = "apple";
    let price = 2.99;
    let colors = ("red", "green");
    log::info!(fruit, price, colors:sval; "info! {fruit} is {price}");
    log::warn!("warn!");
    log::error!("error!");

    // Log manually.
    let logger = logger_provider.logger("test");
    let mut record = logger.create_log_record();
    record.set_severity_number(Severity::Fatal);
    record.add_attribute(semcov::trace::EXCEPTION_TYPE, "Foo");
    record.add_attribute(semcov::trace::EXCEPTION_MESSAGE, "Foo broke");
    record.add_attribute(semcov::trace::EXCEPTION_STACKTRACE, "A stack trace");
    logger.emit(record);

    // Log inside a span.
    tracer.in_span("span_with_logs", |_cx| {
        log::info!("with span");
    });

    // Force export before exit.
    logger_provider.shutdown().unwrap();
    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
