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

fn main_impl<C>(client: C, span_name: &'static str)
where
    C: HttpClient + 'static,
{
    env_logger::init();

    let instrumentation_key =
        env::var("INSTRUMENTATION_KEY").expect("env var INSTRUMENTATION_KEY should exist");

    let (tracer, _uninstall) =
        opentelemetry_application_insights::new_pipeline(instrumentation_key)
            .with_client(client)
            .install();

    tracer.in_span(span_name, |_cx| {});
}

#[cfg_attr(feature = "reqwest-client", tokio::main)]
#[cfg(feature = "reqwest-client")]
async fn main() {
    main_impl(reqwest::Client::new(), "reqwest-client");
}

#[cfg_attr(feature = "surf-client", async_std::main)]
#[cfg(feature = "surf-client")]
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
