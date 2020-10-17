//! Use this example the test the different HTTP clients.
//!
//! Depending on the client, this will use either the tokio, async-std or no async runtime.
//!
//! ```text
//! cargo run --example http_clients --features reqwest-blocking-client
//! cargo run --example http_clients --features reqwest-client
//! cargo run --example http_clients --features surf-client,opentelemetry/async-std
//! ```
use opentelemetry::api::trace::Tracer;
use opentelemetry_application_insights::HttpClient;
use std::env;

#[allow(dead_code)]
fn main_impl<C>(client: C, span_name: &'static str)
where
    C: HttpClient + 'static,
{
    println!("testing: {}", span_name);

    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let (tracer, _uninstall) =
        opentelemetry_application_insights::new_pipeline(instrumentation_key)
            .with_client(client)
            .install();

    tracer.in_span(span_name, |_cx| {});
}

#[cfg_attr(
    all(feature = "reqwest-client", not(feature = "surf-client")),
    tokio::main
)]
#[cfg(all(feature = "reqwest-client", not(feature = "surf-client")))]
async fn main() {
    main_impl(reqwest::Client::new(), "reqwest-client");
}

#[cfg_attr(
    all(feature = "surf-client", not(feature = "reqwest-client")),
    async_std::main
)]
#[cfg(all(feature = "surf-client", not(feature = "reqwest-client")))]
async fn main() {
    main_impl(surf::Client::new(), "surf-client");
}

#[cfg(all(
    feature = "reqwest-blocking-client",
    not(any(feature = "reqwest-client", feature = "surf-client"))
))]
fn main() {
    main_impl(reqwest::blocking::Client::new(), "reqwest-blocking-client");
}

#[cfg(all(
    feature = "reqwest-blocking-client",
    feature = "reqwest-client",
    feature = "surf-client"
))]
fn main() {
    // Selectively enable one of the clients to test it.
}
