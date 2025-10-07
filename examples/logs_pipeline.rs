use opentelemetry::logs::LogRecord;
use opentelemetry_semantic_conventions as semcov;
use std::error::Error;

#[derive(Debug)]
struct ErrorAsExceptionLogProcessor;

impl opentelemetry_sdk::logs::LogProcessor for ErrorAsExceptionLogProcessor {
    fn emit(
        &self,
        data: &mut opentelemetry_sdk::logs::SdkLogRecord,
        _instrumentation: &opentelemetry::InstrumentationScope,
    ) {
        if let Some(severity) = data.severity_number() {
            if severity >= opentelemetry::logs::Severity::Error {
                // TODO: Check if exception attributes are already present
                data.add_attribute(semcov::attribute::EXCEPTION_TYPE, "error");
                if let Some(body) = data.body() {
                    data.add_attribute(
                        semcov::attribute::EXCEPTION_MESSAGE,
                        any_value_to_string(body),
                    );
                }
            }
        }
    }

    fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        Ok(())
    }
}

fn any_value_to_string(v: &opentelemetry::logs::AnyValue) -> String {
    match v {
        opentelemetry::logs::AnyValue::String(v) => v.to_string(),
        _ => format!("{:?}", v).into(),
    }
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = reqwest::blocking::Client::new();

    let exporter = opentelemetry_application_insights::Exporter::new_from_env(client)?;

    let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
        .with_log_processor(ErrorAsExceptionLogProcessor)
        .with_batch_exporter(exporter)
        .build();
    let otel_log_appender =
        opentelemetry_appender_log::OpenTelemetryLogBridge::new(&logger_provider);
    log::set_boxed_logger(Box::new(otel_log_appender))?;
    log::set_max_level(log::Level::Info.to_level_filter());

    // Log via `log` crate.
    let fruit = "apple";
    let price = 2.99;
    let colors = ("red", "green");
    log::info!(fruit, price, colors:sval; "info! {fruit} is {price}");
    log::warn!("warn!");
    log::error!("error!");

    // Force export before exit.
    logger_provider.shutdown()?;

    Ok(())
}
