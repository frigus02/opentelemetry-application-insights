use crate::{
    models::QuickPulseEnvelope,
    uploader::{self, PostOrPing},
    Exporter,
};
use futures_util::{stream, StreamExt as _};
use opentelemetry::runtime::{RuntimeChannel, TrySend};
use opentelemetry_http::HttpClient;
use std::{time::Duration, time::SystemTime};

const MAX_POST_WAIT_TIME: Duration = Duration::from_secs(20);
const MAX_PING_WAIT_TIME: Duration = Duration::from_secs(60);
const FALLBACK_INTERVAL: Duration = Duration::from_secs(60);
const PING_INTERVAL: Duration = Duration::from_secs(5);
const POST_INTERVAL: Duration = Duration::from_secs(1);
const TICK_INTERVAL: Duration = Duration::from_secs(1);

/// Live metrics
#[derive(Debug)]
pub struct QuickPulseManager<R: RuntimeChannel<()>> {
    message_sender: R::Sender,
}

enum Message {
    Tick,
    End,
}

impl<R: RuntimeChannel<()>> QuickPulseManager<R> {
    /// Start live metrics
    pub fn new<C: HttpClient + 'static>(exporter: Exporter<C>, runtime: R) -> QuickPulseManager<R> {
        let (message_sender, message_receiver) = runtime.batch_message_channel(0);
        let ticker = runtime.interval(TICK_INTERVAL).map(|_| Message::Tick);

        let mut messages = Box::pin(stream::select(
            message_receiver.map(|_| Message::End),
            ticker,
        ));
        runtime.spawn(Box::pin(async move {
            let mut next_action_time = SystemTime::UNIX_EPOCH;
            let mut is_collecting = false;
            let mut last_success_time = SystemTime::UNIX_EPOCH;
            let mut redirected_host: Option<http::Uri> = None;
            let mut polling_interval_hint: Option<Duration> = None;
            while let Some(Message::Tick) = messages.next().await {
                let now = SystemTime::now();
                if next_action_time < now {
                    continue;
                }

                // TODO: collect metrics
                // TODO: clear buffer
                let envelope = QuickPulseEnvelope {
                    documents: Vec::new(),
                    instance: "".into(),
                    role_name: "".into(),
                    instrumentation_key: exporter.instrumentation_key.clone(),
                    invariant_version: 1,
                    machine_name: "".into(),
                    metrics: Vec::new(),
                    stream_id: "".into(),
                    timestamp: "".into(),
                    version: "".into(),
                };

                let res = uploader::send_quick_pulse(
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
                    last_success_time = now;
                    is_collecting = res.should_post;
                    if res.redirected_host.is_some() {
                        redirected_host = res.redirected_host;
                    }
                    if res.polling_interval_hint.is_some() {
                        polling_interval_hint = res.polling_interval_hint;
                    }
                    true
                } else {
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

                next_action_time = now + current_timeout;
            }
        }));

        QuickPulseManager { message_sender }
    }
}

impl<R: RuntimeChannel<()>> Drop for QuickPulseManager<R> {
    fn drop(&mut self) {
        if let Err(err) = self.message_sender.try_send(()) {
            opentelemetry::global::handle_error(opentelemetry::metrics::MetricsError::Other(
                err.to_string(),
            ));
        }
    }
}
