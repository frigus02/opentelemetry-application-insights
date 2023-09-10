use crate::{
    models::{QuickPulseEnvelope, QuickPulseMetric, RemoteDependencyData, RequestData},
    trace::{get_duration, EVENT_NAME_EXCEPTION},
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
    trace::{SpanKind, TraceError},
    Context,
};
use opentelemetry_http::HttpClient;
use std::{fmt::Debug, time::Duration, time::SystemTime};
use sysinfo::{CpuExt as _, System, SystemExt as _};

const CHANNEL_CAPACITY: usize = 100;
const MAX_POST_WAIT_TIME: Duration = Duration::from_secs(20);
const MAX_PING_WAIT_TIME: Duration = Duration::from_secs(60);
const FALLBACK_INTERVAL: Duration = Duration::from_secs(60);
const PING_INTERVAL: Duration = Duration::from_secs(5);
const POST_INTERVAL: Duration = Duration::from_secs(1);

const METRIC_PROCESSOR_TIME: &str = "\\Processor(_Total)\\% Processor Time";
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

struct MetricsCollector {
    system: System,
    request_count: u32,
    request_failed_count: u32,
    request_duration: Duration,
    dependency_count: u32,
    dependency_failed_count: u32,
    dependency_duration: Duration,
    exception_count: u32,
    last_collection_time: SystemTime,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            system: System::new(),
            request_count: 0,
            request_failed_count: 0,
            request_duration: Duration::default(),
            dependency_count: 0,
            dependency_failed_count: 0,
            dependency_duration: Duration::default(),
            exception_count: 0,
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
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                let data: RemoteDependencyData = (&span).into();
                self.dependency_count += 1;
                if let Some(false) = data.success {
                    self.dependency_failed_count += 1;
                }
                self.dependency_duration += get_duration(&span);
            }
        }

        for event in span.events.iter() {
            if event.name.as_ref() == EVENT_NAME_EXCEPTION {
                //let data: ExceptionData = event.into();
                self.exception_count += 1;
            }
        }
    }

    fn collect(&mut self) -> Vec<QuickPulseMetric> {
        let mut result = Vec::new();
        self.collect_cpu_usage(&mut result);
        self.collect_requests_dependencies_exceptions(&mut result);
        self.reset();
        result
    }

    fn collect_cpu_usage(&mut self, metrics: &mut Vec<QuickPulseMetric>) {
        self.system.refresh_cpu();
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

#[derive(Debug)]
pub enum Message {
    ProcessSpan(SpanData),
    Stop,
    Send,
}

impl<R: RuntimeChannel<Message> + Debug> QuickPulseManager<R> {
    /// Start live metrics
    pub fn new<C: HttpClient + 'static>(exporter: Exporter<C>, runtime: R) -> QuickPulseManager<R> {
        let (message_sender, message_receiver) = runtime.batch_message_channel(CHANNEL_CAPACITY);
        let delay_runtime = runtime.clone();
        runtime.spawn(Box::pin(async move {
            let mut is_collecting = false;
            let mut last_success_time = SystemTime::now();
            let mut redirected_host: Option<http::Uri> = None;
            let mut polling_interval_hint: Option<Duration> = None;
            let stream_id = format!("{:032x}", RandomIdGenerator::default().new_trace_id());
            let mut metrics_colllector = MetricsCollector::new();

            let message_receiver = message_receiver.fuse();
            pin_mut!(message_receiver);
            let mut delay = delay_runtime.delay(PING_INTERVAL).fuse();

            loop {
                let msg = select_biased! {
                    msg = message_receiver.next() => msg.unwrap_or(Message::Stop),
                    _ = delay => Message::Send,
                };
                match msg {
                    Message::ProcessSpan(span) => {
                        if is_collecting {
                            metrics_colllector.count_span(span);
                        }
                        continue;
                    },
                    Message::Stop => break,
                    Message::Send => {
                        // upload
                    }
                }

                println!("[QPS] Tick");

                let metrics = if is_collecting {
                    metrics_colllector.collect()
                } else {
                    Vec::new()
                };

                println!("[QPS] Action is_collecting={}", is_collecting);

                let now = SystemTime::now();
                let now_ms = now
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let envelope = QuickPulseEnvelope {
                    documents: Vec::new(),
                    metrics,
                    invariant_version: 1,
                    timestamp: format!("/Date({})/", now_ms),
                    version: None,
                    stream_id: stream_id.clone(),
                    machine_name: "Unknown".into(),
                    instance: "Unknown".into(),
                    role_name: None,
                };

                let res = uploader_quick_pulse::send(
                    exporter.client.as_ref(),
                    redirected_host
                        .as_ref()
                        .unwrap_or(&exporter.live_metrics_endpoint),
                    &exporter.instrumentation_key,
                    if is_collecting {
                        PostOrPing::Post
                    } else {
                        PostOrPing::Ping
                    },
                    envelope,
                )
                .await
                .map_err(|_| ());
                let last_send_succeeded = if let Ok(res) = res {
                    println!(
                        "[QPS] Success should_post={} redirected_host={:?} polling_interval_hint={:?}",
                        res.should_post, res.redirected_host, res.polling_interval_hint
                    );
                    last_success_time = now;
                    is_collecting = res.should_post;
                    if is_collecting {
                        // Reset last collection time to get accurate metrics on next collection.
                        metrics_colllector.reset();
                    }
                    if res.redirected_host.is_some() {
                        redirected_host = res.redirected_host;
                    }
                    if res.polling_interval_hint.is_some() {
                        polling_interval_hint = res.polling_interval_hint;
                    }
                    true
                } else {
                    println!("[QPS] Failure");
                    false
                };

                let mut current_timeout = if is_collecting {
                    POST_INTERVAL
                } else {
                    polling_interval_hint.unwrap_or(PING_INTERVAL)
                };
                if !last_send_succeeded {
                    let time_since_last_success = now
                        .duration_since(last_success_time)
                        .unwrap_or(Duration::MAX);
                    if is_collecting && time_since_last_success >= MAX_POST_WAIT_TIME {
                        // Haven't posted successfully in 20 seconds, so wait 60 seconds and ping
                        is_collecting = false;
                        current_timeout = FALLBACK_INTERVAL;
                    } else if !is_collecting && time_since_last_success >= MAX_PING_WAIT_TIME {
                        // Haven't pinged successfully in 60 seconds, so wait another 60 seconds
                        current_timeout = FALLBACK_INTERVAL;
                    }
                }

                println!("[QPS] Next in {:?}", current_timeout);
                delay = delay_runtime.delay(current_timeout).fuse();
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
