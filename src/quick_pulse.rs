use crate::{
    models::{context_tag_keys, QuickPulseEnvelope, QuickPulseMetric},
    tags::get_tags_from_attrs,
    trace::{get_duration, is_remote_dependency_success, is_request_success, EVENT_NAME_EXCEPTION},
    uploader_quick_pulse::{self, PostOrPing},
    Error,
};
use futures_util::{pin_mut, select_biased, FutureExt as _, StreamExt as _};
use opentelemetry::{
    trace::{SpanKind, TraceResult},
    Context, Key,
};
use opentelemetry_http::HttpClient;
use opentelemetry_sdk::{
    export::trace::SpanData,
    runtime::{RuntimeChannel, TrySend},
    trace::{IdGenerator as _, RandomIdGenerator, Span, SpanProcessor},
    Resource,
};
use opentelemetry_semantic_conventions as semcov;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
    time::SystemTime,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

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

pub(crate) struct QuickPulseManager<R: RuntimeChannel> {
    is_collecting: Arc<AtomicBool>,
    metrics_collector: Arc<Mutex<MetricsCollector>>,
    message_sender: R::Sender<Message>,
}

impl<R: RuntimeChannel> std::fmt::Debug for QuickPulseManager<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuickPulseManager").finish()
    }
}

#[derive(Debug)]
enum Message {
    Send,
    Stop,
}

impl<R: RuntimeChannel> QuickPulseManager<R> {
    pub(crate) fn new<C: HttpClient + 'static>(
        client: Arc<C>,
        live_metrics_endpoint: http::Uri,
        instrumentation_key: String,
        resource: Resource,
        runtime: R,
    ) -> QuickPulseManager<R> {
        let (message_sender, message_receiver) = runtime.batch_message_channel(1);
        let delay_runtime = runtime.clone();
        let is_collecting_outer = Arc::new(AtomicBool::new(false));
        let is_collecting = is_collecting_outer.clone();
        let metrics_collector_outer = Arc::new(Mutex::new(MetricsCollector::new()));
        let metrics_collector = metrics_collector_outer.clone();
        runtime.spawn(Box::pin(async move {
            let mut sender =
                QuickPulseSender::new(client, live_metrics_endpoint, instrumentation_key, resource);

            let message_receiver = message_receiver.fuse();
            pin_mut!(message_receiver);
            let mut send_delay = delay_runtime.delay(PING_INTERVAL).fuse();

            loop {
                let msg = select_biased! {
                    msg = message_receiver.next() => msg.unwrap_or(Message::Stop),
                    _ = send_delay => Message::Send
                };
                match msg {
                    Message::Send => {
                        let curr_is_collecting = is_collecting.load(Ordering::SeqCst);
                        let metrics = if curr_is_collecting {
                            metrics_collector.lock().unwrap().collect_and_reset()
                        } else {
                            Vec::new()
                        };
                        let (next_is_collecting, next_timeout) =
                            sender.send(curr_is_collecting, metrics).await;
                        if curr_is_collecting != next_is_collecting {
                            is_collecting.store(next_is_collecting, Ordering::SeqCst);
                            if next_is_collecting {
                                // Reset last collection time to get accurate metrics on next collection.
                                metrics_collector.lock().unwrap().reset();
                            }
                        }
                        send_delay = delay_runtime.delay(next_timeout).fuse();
                    }
                    Message::Stop => break,
                }
            }
        }));

        QuickPulseManager {
            is_collecting: is_collecting_outer,
            metrics_collector: metrics_collector_outer,
            message_sender,
        }
    }
}

impl<R: RuntimeChannel> SpanProcessor for QuickPulseManager<R> {
    fn on_start(&self, _span: &mut Span, _cx: &Context) {}

    fn on_end(&self, span: SpanData) {
        if self.is_collecting.load(Ordering::SeqCst) {
            self.metrics_collector.lock().unwrap().count_span(span);
        }
    }

    fn force_flush(&self) -> TraceResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> TraceResult<()> {
        self.message_sender
            .try_send(Message::Stop)
            .map_err(Error::QuickPulseShutdown)?;
        Ok(())
    }
}

impl<R: RuntimeChannel> Drop for QuickPulseManager<R> {
    fn drop(&mut self) {
        if let Err(err) = self.shutdown() {
            opentelemetry::global::handle_error(err);
        }
    }
}

struct QuickPulseSender<C: HttpClient + 'static> {
    client: Arc<C>,
    host: http::Uri,
    instrumentation_key: String,
    last_success_time: SystemTime,
    polling_interval_hint: Option<Duration>,
    version: Option<String>,
    stream_id: String,
    machine_name: String,
    instance: String,
    role_name: Option<String>,
}

impl<C: HttpClient + 'static> QuickPulseSender<C> {
    fn new(
        client: Arc<C>,
        host: http::Uri,
        instrumentation_key: String,
        resource: Resource,
    ) -> Self {
        let mut tags = get_tags_from_attrs(resource.iter());
        let machine_name = resource
            .get(Key::from_static_str(semcov::resource::HOST_NAME))
            .map(|v| v.as_str().into_owned())
            .unwrap_or_else(|| "Unknown".into());
        Self {
            client,
            host,
            instrumentation_key,
            last_success_time: SystemTime::now(),
            polling_interval_hint: None,
            version: tags.remove(context_tag_keys::INTERNAL_SDK_VERSION),
            stream_id: format!("{:032x}", RandomIdGenerator::default().new_trace_id()),
            role_name: tags.remove(context_tag_keys::CLOUD_ROLE),
            instance: tags
                .remove(context_tag_keys::CLOUD_ROLE_INSTANCE)
                .unwrap_or_else(|| machine_name.clone()),
            machine_name,
        }
    }

    async fn send(
        &mut self,
        is_collecting: bool,
        metrics: Vec<QuickPulseMetric>,
    ) -> (bool, Duration) {
        let now = SystemTime::now();
        let now_ms = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let envelope = QuickPulseEnvelope {
            metrics,
            invariant_version: 1,
            timestamp: format!("/Date({})/", now_ms),
            version: self.version.clone(),
            stream_id: self.stream_id.clone(),
            machine_name: self.machine_name.clone(),
            instance: self.instance.clone(),
            role_name: self.role_name.clone(),
        };

        let res = uploader_quick_pulse::send(
            self.client.as_ref(),
            &self.host,
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
            self.last_success_time = now;
            if let Some(redirected_host) = res.redirected_host {
                self.host = redirected_host;
            }
            if res.polling_interval_hint.is_some() {
                self.polling_interval_hint = res.polling_interval_hint;
            }
            (true, res.should_post)
        } else {
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
    request_count: usize,
    request_failed_count: usize,
    request_duration: Duration,
    dependency_count: usize,
    dependency_failed_count: usize,
    dependency_duration: Duration,
    exception_count: usize,
    last_collection_time: SystemTime,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            system: System::new(),
            system_refresh_kind: RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(MemoryRefreshKind::new().with_ram()),
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
                self.request_count += 1;
                if !is_request_success(&span) {
                    self.request_failed_count += 1;
                }
                self.request_duration += get_duration(&span);
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                self.dependency_count += 1;
                if let Some(false) = is_remote_dependency_success(&span) {
                    self.dependency_failed_count += 1;
                }
                self.dependency_duration += get_duration(&span);
            }
        }

        for event in span.events.iter() {
            if event.name == EVENT_NAME_EXCEPTION {
                self.exception_count += 1;
            }
        }
    }

    fn collect_and_reset(&mut self) -> Vec<QuickPulseMetric> {
        let mut metrics = Vec::new();
        self.system.refresh_specifics(self.system_refresh_kind);
        self.collect_cpu_usage(&mut metrics);
        self.collect_memory_usage(&mut metrics);
        self.collect_requests_dependencies_exceptions(&mut metrics);
        self.reset();
        metrics
    }

    fn collect_cpu_usage(&mut self, metrics: &mut Vec<QuickPulseMetric>) {
        let mut cpu_usage = 0.;
        for cpu in self.system.cpus() {
            cpu_usage += f64::from(cpu.cpu_usage());
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
            value: self.system.used_memory() as f64,
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
            value: self.request_count as f64 / elapsed_seconds as f64,
            weight: 1,
        });
        metrics.push(QuickPulseMetric {
            name: METRIC_REQUEST_FAILURE_RATE,
            value: self.request_failed_count as f64 / elapsed_seconds as f64,
            weight: 1,
        });
        if self.request_count > 0 {
            metrics.push(QuickPulseMetric {
                name: METRIC_REQUEST_DURATION,
                value: self.request_duration.as_millis() as f64 / self.request_count as f64,
                weight: 1,
            });
        }

        metrics.push(QuickPulseMetric {
            name: METRIC_DEPENDENCY_RATE,
            value: self.dependency_count as f64 / elapsed_seconds as f64,
            weight: 1,
        });
        metrics.push(QuickPulseMetric {
            name: METRIC_DEPENDENCY_FAILURE_RATE,
            value: self.dependency_failed_count as f64 / elapsed_seconds as f64,
            weight: 1,
        });
        if self.dependency_count > 0 {
            metrics.push(QuickPulseMetric {
                name: METRIC_DEPENDENCY_DURATION,
                value: self.dependency_duration.as_millis() as f64 / self.dependency_count as f64,
                weight: 1,
            });
        }

        metrics.push(QuickPulseMetric {
            name: METRIC_EXCEPTION_RATE,
            value: self.exception_count as f64 / elapsed_seconds as f64,
            weight: 1,
        });
    }
}
