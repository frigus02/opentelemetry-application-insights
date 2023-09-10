use crate::{
    models::{
        EventData, ExceptionData, MessageData, Properties, QuickPulseDocument,
        QuickPulseDocumentProperty, QuickPulseDocumentType, QuickPulseEnvelope, QuickPulseMetric,
        RemoteDependencyData, RequestData, SeverityLevel,
    },
    trace::{get_duration, EVENT_NAME_CUSTOM, EVENT_NAME_EXCEPTION},
    uploader_quick_pulse::{self, PostOrPing},
    Exporter,
};
use futures_util::{pin_mut, select_biased, FutureExt as _, StreamExt as _};
use opentelemetry::{
    runtime::{RuntimeChannel, TrySend},
    sdk::{
        export::trace::SpanData,
        trace::{IdGenerator as _, RandomIdGenerator, Span, SpanProcessor},
    },
    trace::{SpanKind, TraceError, TraceId},
    Context,
};
use opentelemetry_http::HttpClient;
use std::{fmt::Debug, sync::Arc, time::Duration, time::SystemTime};
use sysinfo::{CpuExt as _, CpuRefreshKind, RefreshKind, System, SystemExt as _};

const CHANNEL_CAPACITY: usize = 100;
const MAX_POST_WAIT_TIME: Duration = Duration::from_secs(20);
const MAX_PING_WAIT_TIME: Duration = Duration::from_secs(60);
const FALLBACK_INTERVAL: Duration = Duration::from_secs(60);
const PING_INTERVAL: Duration = Duration::from_secs(5);
const POST_INTERVAL: Duration = Duration::from_secs(1);

const METRIC_PROCESSOR_TIME: &str = "\\Processor(_Total)\\% Processor Time";
const METRIC_COMMITTED_BYTES: &str = "\\Memory\\Committed Bytes";
const METRIC_REQUEST_RATE: &str = "\\ApplicationInsights\\Requests/Sec";
const METRIC_REQUEST_FAILURE_RATE: &str = "\\ApplicationInsights\\Requests Failed/Sec";
const METRIC_REQUEST_DURATION: &str = "\\ApplicationInsights\\Request Duration";
const METRIC_DEPENDENCY_RATE: &str = "\\ApplicationInsights\\Dependency Calls/Sec";
const METRIC_DEPENDENCY_FAILURE_RATE: &str = "\\ApplicationInsights\\Dependency Calls Failed/Sec";
const METRIC_DEPENDENCY_DURATION: &str = "\\ApplicationInsights\\Dependency Call Duration";
const METRIC_EXCEPTION_RATE: &str = "\\ApplicationInsights\\Exceptions/Sec";

/// Live metrics
#[derive(Debug)]
pub struct QuickPulseManager<R: RuntimeChannel<Message> + Debug> {
    message_sender: R::Sender,
}

#[derive(Debug)]
pub enum Message {
    ProcessSpan(SpanData),
    Send,
    Stop,
}

impl<R: RuntimeChannel<Message> + Debug> QuickPulseManager<R> {
    /// Start live metrics
    pub fn new<C: HttpClient + 'static>(exporter: Exporter<C>, runtime: R) -> QuickPulseManager<R> {
        let (message_sender, message_receiver) = runtime.batch_message_channel(CHANNEL_CAPACITY);
        let delay_runtime = runtime.clone();
        runtime.spawn(Box::pin(async move {
            let mut is_collecting = false;
            let mut metrics_colllector = MetricsCollector::new();
            let mut sender = QuickPulseSender::new(
                exporter.client,
                exporter.live_metrics_endpoint,
                exporter.instrumentation_key,
            );

            let message_receiver = message_receiver.fuse();
            pin_mut!(message_receiver);
            let mut send_delay = delay_runtime.delay(PING_INTERVAL).fuse();

            loop {
                let msg = select_biased! {
                    msg = message_receiver.next() => msg.unwrap_or(Message::Stop),
                    _ = send_delay => Message::Send,
                };
                match msg {
                    Message::ProcessSpan(span) => {
                        if is_collecting {
                            metrics_colllector.count_span(span);
                        }
                    }
                    Message::Send => {
                        let (metrics, documents) = if is_collecting {
                            metrics_colllector.collect()
                        } else {
                            (Vec::new(), Vec::new())
                        };
                        let (next_is_collecting, next_timeout) =
                            sender.send(is_collecting, metrics, documents).await;
                        if !is_collecting && next_is_collecting {
                            // Reset last collection time to get accurate metrics on next collection.
                            metrics_colllector.reset();
                        }
                        is_collecting = next_is_collecting;
                        send_delay = delay_runtime.delay(next_timeout).fuse();
                    }
                    Message::Stop => break,
                }
            }
        }));

        QuickPulseManager { message_sender }
    }
}

impl<R: RuntimeChannel<Message> + Debug> SpanProcessor for QuickPulseManager<R> {
    fn on_start(&self, _span: &mut Span, _cx: &Context) {}

    fn on_end(&self, span: SpanData) {
        if let Err(err) = self.message_sender.try_send(Message::ProcessSpan(span)) {
            opentelemetry::global::handle_error(TraceError::Other(err.into()));
        }
    }

    fn force_flush(&self) -> Result<(), TraceError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), TraceError> {
        self.message_sender
            .try_send(Message::Stop)
            .map_err(|err| TraceError::Other(err.into()))?;
        Ok(())
    }
}

impl<R: RuntimeChannel<Message> + Debug> Drop for QuickPulseManager<R> {
    fn drop(&mut self) {
        if let Err(err) = self.shutdown() {
            opentelemetry::global::handle_error(err);
        }
    }
}

struct QuickPulseSender<C: HttpClient + 'static> {
    client: Arc<C>,
    host: Arc<http::Uri>,
    instrumentation_key: String,
    last_success_time: SystemTime,
    polling_interval_hint: Option<Duration>,
    stream_id: String,
}

impl<C: HttpClient + 'static> QuickPulseSender<C> {
    fn new(client: Arc<C>, host: Arc<http::Uri>, instrumentation_key: String) -> Self {
        Self {
            client,
            host,
            instrumentation_key,
            last_success_time: SystemTime::now(),
            polling_interval_hint: None,
            stream_id: format!("{:032x}", RandomIdGenerator::default().new_trace_id()),
        }
    }

    async fn send(
        &mut self,
        is_collecting: bool,
        metrics: Vec<QuickPulseMetric>,
        documents: Vec<QuickPulseDocument>,
    ) -> (bool, Duration) {
        let now = SystemTime::now();
        let now_ms = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let envelope = QuickPulseEnvelope {
            documents,
            metrics,
            invariant_version: 1,
            timestamp: format!("/Date({})/", now_ms),
            version: None,
            stream_id: self.stream_id.clone(),
            machine_name: "Unknown".into(),
            instance: "Unknown".into(),
            role_name: None,
        };

        let res = uploader_quick_pulse::send(
            self.client.as_ref(),
            self.host.as_ref(),
            &self.instrumentation_key,
            if is_collecting {
                PostOrPing::Post
            } else {
                PostOrPing::Ping
            },
            envelope,
        )
        .await;
        let (last_send_succeeded, mut next_is_collecting) = if let Ok(res) = res {
            println!(
                "[QPS] Success should_post={} redirected_host={:?} polling_interval_hint={:?}",
                res.should_post, res.redirected_host, res.polling_interval_hint
            );
            self.last_success_time = now;
            if let Some(redirected_host) = res.redirected_host {
                self.host = Arc::new(redirected_host);
            }
            if res.polling_interval_hint.is_some() {
                self.polling_interval_hint = res.polling_interval_hint;
            }
            (true, res.should_post)
        } else {
            println!("[QPS] Failure");
            (false, is_collecting)
        };

        let mut next_timeout = if next_is_collecting {
            POST_INTERVAL
        } else {
            self.polling_interval_hint.unwrap_or(PING_INTERVAL)
        };
        if !last_send_succeeded {
            let time_since_last_success = now
                .duration_since(self.last_success_time)
                .unwrap_or(Duration::MAX);
            if next_is_collecting && time_since_last_success >= MAX_POST_WAIT_TIME {
                // Haven't posted successfully in 20 seconds, so wait 60 seconds and ping
                next_is_collecting = false;
                next_timeout = FALLBACK_INTERVAL;
            } else if !next_is_collecting && time_since_last_success >= MAX_PING_WAIT_TIME {
                // Haven't pinged successfully in 60 seconds, so wait another 60 seconds
                next_timeout = FALLBACK_INTERVAL;
            }
        }

        (next_is_collecting, next_timeout)
    }
}

struct MetricsCollector {
    system: System,
    system_refresh_kind: RefreshKind,
    request_count: u32,
    request_failed_count: u32,
    request_duration: Duration,
    dependency_count: u32,
    dependency_failed_count: u32,
    dependency_duration: Duration,
    exception_count: u32,
    documents: Vec<QuickPulseDocument>,
    last_collection_time: SystemTime,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            system: System::new(),
            system_refresh_kind: RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(),
            request_count: 0,
            request_failed_count: 0,
            request_duration: Duration::default(),
            dependency_count: 0,
            dependency_failed_count: 0,
            dependency_duration: Duration::default(),
            exception_count: 0,
            documents: Vec::new(),
            last_collection_time: SystemTime::now(),
        }
    }

    fn reset(&mut self) {
        self.request_count = 0;
        self.request_failed_count = 0;
        self.request_duration = Duration::default();
        self.dependency_count = 0;
        self.dependency_failed_count = 0;
        self.dependency_duration = Duration::default();
        self.exception_count = 0;
        self.documents.clear();
        self.last_collection_time = SystemTime::now();
    }

    fn count_span(&mut self, span: SpanData) {
        // https://github.com/microsoft/ApplicationInsights-node.js/blob/aaafbfd8ffbc454d4a5c30cda4492891410b9f66/TelemetryProcessors/PerformanceMetricsTelemetryProcessor.ts#L6
        match span.span_kind {
            SpanKind::Server | SpanKind::Consumer => {
                let data: RequestData = (&span).into();
                self.request_count += 1;
                if !data.success {
                    self.request_failed_count += 1;
                }
                self.request_duration += get_duration(&span);
                self.documents
                    .push((span.span_context.trace_id(), data).into());
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                let data: RemoteDependencyData = (&span).into();
                self.dependency_count += 1;
                if let Some(false) = data.success {
                    self.dependency_failed_count += 1;
                }
                self.dependency_duration += get_duration(&span);
                self.documents
                    .push((span.span_context.trace_id(), data).into());
            }
        }

        for event in span.events.iter() {
            match event.name.as_ref() {
                x if x == EVENT_NAME_CUSTOM => {
                    self.documents
                        .push((span.span_context.trace_id(), EventData::from(event)).into());
                }
                x if x == EVENT_NAME_EXCEPTION => {
                    self.exception_count += 1;
                    self.documents
                        .push((span.span_context.trace_id(), ExceptionData::from(event)).into());
                }
                _ => {
                    self.documents
                        .push((span.span_context.trace_id(), MessageData::from(event)).into());
                }
            };
        }
    }

    fn collect(&mut self) -> (Vec<QuickPulseMetric>, Vec<QuickPulseDocument>) {
        let mut metrics = Vec::new();
        self.system.refresh_specifics(self.system_refresh_kind);
        self.collect_cpu_usage(&mut metrics);
        self.collect_memory_usage(&mut metrics);
        self.collect_requests_dependencies_exceptions(&mut metrics);
        let documents = self.documents.split_off(0);
        self.reset();
        (metrics, documents)
    }

    fn collect_cpu_usage(&mut self, metrics: &mut Vec<QuickPulseMetric>) {
        let mut cpu_usage = 0.;
        for cpu in self.system.cpus() {
            cpu_usage += cpu.cpu_usage();
        }
        metrics.push(QuickPulseMetric {
            name: METRIC_PROCESSOR_TIME,
            value: cpu_usage,
            weight: 1,
        });
    }

    fn collect_memory_usage(&mut self, metrics: &mut Vec<QuickPulseMetric>) {
        metrics.push(QuickPulseMetric {
            name: METRIC_COMMITTED_BYTES,
            value: self.system.used_memory() as f32,
            weight: 1,
        });
    }

    fn collect_requests_dependencies_exceptions(&mut self, metrics: &mut Vec<QuickPulseMetric>) {
        let elapsed_seconds = SystemTime::now()
            .duration_since(self.last_collection_time)
            .unwrap_or_default()
            .as_secs();
        if elapsed_seconds == 0 {
            return;
        }

        metrics.push(QuickPulseMetric {
            name: METRIC_REQUEST_RATE,
            value: self.request_count as f32 / elapsed_seconds as f32,
            weight: 1,
        });
        metrics.push(QuickPulseMetric {
            name: METRIC_REQUEST_FAILURE_RATE,
            value: self.request_failed_count as f32 / elapsed_seconds as f32,
            weight: 1,
        });
        if self.request_count > 0 {
            metrics.push(QuickPulseMetric {
                name: METRIC_REQUEST_DURATION,
                value: self.request_duration.as_millis() as f32 / self.request_count as f32,
                weight: 1,
            });
        }

        metrics.push(QuickPulseMetric {
            name: METRIC_DEPENDENCY_RATE,
            value: self.dependency_count as f32 / elapsed_seconds as f32,
            weight: 1,
        });
        metrics.push(QuickPulseMetric {
            name: METRIC_DEPENDENCY_FAILURE_RATE,
            value: self.dependency_failed_count as f32 / elapsed_seconds as f32,
            weight: 1,
        });
        if self.dependency_count > 0 {
            metrics.push(QuickPulseMetric {
                name: METRIC_DEPENDENCY_DURATION,
                value: self.dependency_duration.as_millis() as f32 / self.dependency_count as f32,
                weight: 1,
            });
        }

        metrics.push(QuickPulseMetric {
            name: METRIC_EXCEPTION_RATE,
            value: self.exception_count as f32 / elapsed_seconds as f32,
            weight: 1,
        });
    }
}

impl From<(TraceId, RequestData)> for QuickPulseDocument {
    fn from((trace_id, value): (TraceId, RequestData)) -> Self {
        let name: String = value.name.map(Into::into).unwrap_or_default();
        Self {
            type_: "RequestTelemetryDocument",
            version: "1.0",
            operation_id: trace_id.to_string(),
            properties: value.properties.map(convert_properties).unwrap_or_default(),
            document_type: QuickPulseDocumentType::Request {
                name: name.clone(),
                success: Some(value.success),
                duration: value.duration,
                response_code: value.response_code.into(),
                operation_name: name,
            },
        }
    }
}

impl From<(TraceId, RemoteDependencyData)> for QuickPulseDocument {
    fn from((trace_id, value): (TraceId, RemoteDependencyData)) -> Self {
        let name: String = value.name.into();
        Self {
            type_: "DependencyTelemetryDocument",
            version: "1.0",
            operation_id: trace_id.to_string(),
            properties: value.properties.map(convert_properties).unwrap_or_default(),
            document_type: QuickPulseDocumentType::Dependency {
                name: name.clone(),
                target: value.target.map(Into::into).unwrap_or_default(),
                success: value.success,
                duration: value.duration,
                result_code: value.result_code.map(Into::into).unwrap_or_default(),
                command_name: value.data.map(Into::into).unwrap_or_default(),
                dependency_type_name: value.type_.map(Into::into).unwrap_or_default(),
                operation_name: name,
            },
        }
    }
}

impl From<(TraceId, EventData)> for QuickPulseDocument {
    fn from((trace_id, value): (TraceId, EventData)) -> Self {
        Self {
            type_: "EventTelemetryDocument",
            version: "1.0",
            operation_id: trace_id.to_string(),
            properties: value.properties.map(convert_properties).unwrap_or_default(),
            document_type: QuickPulseDocumentType::Event {
                name: value.name.into(),
            },
        }
    }
}

impl From<(TraceId, ExceptionData)> for QuickPulseDocument {
    fn from((trace_id, mut value): (TraceId, ExceptionData)) -> Self {
        let exception = value
            .exceptions
            .pop()
            .expect("contains always 1 exception detail");
        Self {
            type_: "ExceptionTelemetryDocument",
            version: "1.0",
            operation_id: trace_id.to_string(),
            properties: value.properties.map(convert_properties).unwrap_or_default(),
            document_type: QuickPulseDocumentType::Exception {
                exception: exception.stack.map(Into::into).unwrap_or_default(),
                exception_message: exception.message.into(),
                exception_type: exception.type_name.into(),
            },
        }
    }
}

impl From<(TraceId, MessageData)> for QuickPulseDocument {
    fn from((trace_id, value): (TraceId, MessageData)) -> Self {
        Self {
            type_: "TraceTelemetryDocument",
            version: "1.0",
            operation_id: trace_id.to_string(),
            properties: value.properties.map(convert_properties).unwrap_or_default(),
            document_type: QuickPulseDocumentType::Trace {
                message: value.message.into(),
                severity_level: value
                    .severity_level
                    .map(|severity_level| match severity_level {
                        SeverityLevel::Verbose => "Verbose",
                        SeverityLevel::Information => "Information",
                        SeverityLevel::Warning => "Warning",
                        SeverityLevel::Error => "Error",
                    })
                    .unwrap_or_default(),
            },
        }
    }
}

fn convert_properties(value: Properties) -> Vec<QuickPulseDocumentProperty> {
    value
        .into_iter()
        .map(|(k, v)| QuickPulseDocumentProperty {
            key: k.into(),
            value: v.into(),
        })
        .collect()
}
