use async_trait::async_trait;
use bytes::Bytes;
use http::{Request, Response};
use num_format::{Locale, ToFormattedString};
use opentelemetry::trace::{Span, SpanKind, Status, Tracer, TracerProvider};
use opentelemetry_application_insights::new_pipeline_from_connection_string;
use opentelemetry_http::{HttpClient, HttpError};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

const CONNECTION_STRING: &str = "InstrumentationKey=0fdcec70-0ce5-4085-89d9-9ae8ead9af66";
const BATCH_SIZE: u64 = 1000;

static STOP: AtomicBool = AtomicBool::new(false);

#[repr(C)]
#[derive(Default)]
struct WorkerStats {
    count: AtomicU64,
    /// We use a padding for the struct to allow each thread to have exclusive access to each WorkerStat
    /// Otherwise, there would be some cpu contention with threads needing to take ownership of the cache lines
    padding: [u64; 15],
}

pub fn test_throughput<F>(func: F)
where
    F: Fn() + Sync + Send + 'static,
{
    ctrlc::set_handler(move || {
        STOP.store(true, Ordering::Release);
    })
    .expect("Error setting Ctrl-C handler");
    opentelemetry::global::set_error_handler(move |err| {
        if !STOP.swap(true, Ordering::AcqRel) {
            eprintln!("OpenTelemetry error: {}", err);
            eprintln!("Suppressing all following errors.");
        }
    })
    .expect("Error setting opentelemetry error handler");

    let num_threads = num_cpus::get_physical();
    println!("Number of threads: {}", num_threads);
    let mut handles = Vec::with_capacity(num_threads);
    let mut worker_stats_vec: Vec<WorkerStats> = Vec::with_capacity(num_threads);
    for _ in 0..num_threads {
        worker_stats_vec.push(WorkerStats::default());
    }
    let worker_stats_shared = Arc::new(worker_stats_vec);
    let worker_stats_shared_monitor = Arc::clone(&worker_stats_shared);

    let func_arc = Arc::new(func);

    let handle_main_thread = thread::spawn(move || {
        let mut total_count_old: u64 = 0;
        loop {
            thread::sleep(Duration::from_secs(1));

            let total_count: u64 = worker_stats_shared_monitor
                .iter()
                .map(|worker_stat| worker_stat.count.load(Ordering::Relaxed))
                .sum();
            let throughput = total_count - total_count_old;
            total_count_old = total_count;
            println!(
                "Throughput: {} iterations/sec",
                throughput.to_formatted_string(&Locale::en),
            );

            if STOP.load(Ordering::Acquire) {
                break;
            }
        }
    });
    handles.push(handle_main_thread);

    for thread_index in 0..num_threads - 1 {
        let worker_stats_shared = Arc::clone(&worker_stats_shared);
        let func_arc_clone = Arc::clone(&func_arc);
        let handle = thread::spawn(move || loop {
            for _ in 0..BATCH_SIZE {
                func_arc_clone();
            }
            worker_stats_shared[thread_index]
                .count
                .fetch_add(BATCH_SIZE, Ordering::Relaxed);
            if STOP.load(Ordering::Acquire) {
                break;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let live_metrics_subscribed: bool = std::env::var("LIVE_METRICS_SUBSCRIBED")
        .expect("env var LIVE_METRICS_SUBSCRIBED")
        .parse()
        .expect("env var LIVE_METRICS_SUBSCRIBED is bool");
    println!("Live metrics subscribed: {}", live_metrics_subscribed);

    let tracer_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
        .expect("connection string is valid")
        .with_client(DummyHttpClient {
            live_metrics_subscribed,
        })
        .with_live_metrics(true)
        .build_batch(opentelemetry::runtime::Tokio);
    let tracer = tracer_provider.tracer("test");

    test_throughput(move || {
        let _span = tracer
            .span_builder("live-metrics")
            .with_kind(SpanKind::Server)
            .start(&tracer);
        let _span = tracer
            .span_builder("live-metrics")
            .with_kind(SpanKind::Server)
            .with_status(Status::error(""))
            .start(&tracer);
        let mut span = tracer
            .span_builder("live-metrics")
            .with_kind(SpanKind::Client)
            .with_status(Status::error(""))
            .start(&tracer);
        let error: Box<dyn std::error::Error> = "An error".into();
        span.record_error(error.as_ref());
    });

    Ok(())
}

#[derive(Debug, Clone)]
pub struct DummyHttpClient {
    pub live_metrics_subscribed: bool,
}

#[async_trait]
impl HttpClient for DummyHttpClient {
    async fn send(&self, req: Request<Vec<u8>>) -> Result<Response<Bytes>, HttpError> {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let is_live_metrics = req.uri().path().contains("QuickPulseService.svc");
        let res = if is_live_metrics {
            Response::builder()
                .status(200)
                .header(
                    "x-ms-qps-subscribed",
                    if self.live_metrics_subscribed {
                        "true"
                    } else {
                        "false"
                    },
                )
                .body(Bytes::new())
                .expect("response is fell formed")
        } else {
            Response::builder()
                .status(200)
                .body(Bytes::from("{}"))
                .expect("response is fell formed")
        };

        Ok(res)
    }
}
