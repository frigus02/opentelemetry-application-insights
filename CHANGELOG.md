# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Add option `.with_resource_attributes_in_events_and_logs(true)`. When enabled, resource attributes are included in events and logs, i.e. Trace, Exception and Event telemetry.

## [0.38.0] - 2025-02-22

- Upgrade `opentelemetry` dependencies to `v0.28`.

  - The `trace` feature turns on `opentelemetry_sdk/experimental_trace_batch_span_processor_with_async_runtime` in this release to avoid breaking API changes and to make this release simpler for me. In the future I hope to align the API with other crates like `opentelemetry-otlp`, which means removing the pipeline API. Examples have already been updated to the new API.

  - If you're using `logs` or `metrics` make sure you use matching combinations of sync/async HTTP clients and runtimes. E.g.:

    - Use `reqwest::blocking::Client` with `.with_batch_exporter(exporter)`. If you're already in an async context, you might need to create the client using `std::thread::spawn(reqwest::blocking::Client::new).join().unwrap()`.
    - Use `reqwest::Client` with `.with_log_processor(opentelemetry_sdk::logs::log_processor_with_async_runtime::BatchLogProcessor::builder(exporter, opentelemetry_sdk::runtime::Tokio).build())`.

  - The `db.system` attribute has been deprecated. You can use `db.system.name` going forward, although the deprecated attribute continues to work.

## [0.37.0] - 2024-11-12

- Upgrade `opentelemetry` dependencies to `v0.27`.
- Upgrade `thiserror` dependency to `v2`.

## [0.36.0] - 2024-10-15

- Upgrade `opentelemetry` dependencies to `v0.26`. Thanks, [sezna@](https://github.com/sezna).

## [0.35.0] - 2024-09-16

- Upgrade `opentelemetry` dependencies to `v0.25`.

## [0.34.0] - 2024-07-20

- Upgrade `opentelemetry` and `opentelemetry_sdk` to `v0.24`.
- Upgrade `opentelemetry-http` to `v0.13`.
- Upgrade `opentelemetry-semantic-conventions` to `v0.16`.
- Upgrade `http` to `1` and `reqwest` to `0.12`.
- Add `trace` feature and enable `trace`, `logs` and `metrics` by default. This mimicks opentelemetry.

## [0.33.0] - 2024-06-09

- Add support for exporting logs, e.g. using one of the `opentelemetry-appender-*` crates.
- Events with the `level` attribute set to `DEBUG`, not convert to a severity level "Verbose" (previously "Information"). This was done to align with Application Insights exporters in other languages.

## [0.32.0] - 2024-05-15

- Upgrade `opentelemetry` and `opentelemetry_sdk` to `v0.23`.
- Upgrade `opentelemetry-http` to `v0.12`.
- Upgrade `opentelemetry-semantic-conventions` to `v0.15`.
  - This removes and deprecates some attributes. All removed/deprecated attributes continue to work. Consider migrating to the new attributes in the future. See "Deprecated attributes" in opentelemetry-application-insights documentation for suitable replacements.
- Set tags Cloud role and Cloud role instance also from resource attributes `k8s.{deployment,replicaset,statefulset,job,cronjob,daemonset,pod}.name`. This matches [the behavior of the JS exporter](https://github.com/Azure/azure-sdk-for-js/blob/c66cad23c4b803719db65cb48a453b0adc13307b/sdk/monitor/monitor-opentelemetry-exporter/src/utils/common.ts#L75-L138).
- Remove option to configure temporality seletor (`with_temporality_selector`). Application Insights supports delta temporality only as defined in the [spec](https://github.com/open-telemetry/opentelemetry-specification/blob/58bfe48eabe887545198d66c43f44071b822373f/specification/metrics/sdk_exporters/otlp.md?plain=1#L46-L47).
- Add support for `ExponentialHistogram` export.
- Add support for span link export.

## [0.31.0] - 2024-05-09

- Change how the tags Could role, Cloud role instance, Application version and Internal SDK version are extracted:

  - Spans no longer extract them from span attributes. They still extract them from resource attributes. And they newly extract them also from instrumentation library attributes.
  - Events newly extract them from resource and instrumentation library attributes.

  In addition metrics no longer extract the tag Authenticated user id from the enduser.id attribute.

- Set tags Device id and Device model based on resource attributes `device.id` and `device.model.name`. This matches [the behavior of the JS exporter](https://github.com/Azure/azure-sdk-for-js/blob/9646e4e3e438fe3a07325989830dd6be35cefc23/sdk/monitor/monitor-opentelemetry-exporter/src/utils/common.ts#L58-L65).

## [0.30.0] - 2024-03-08

- Upgrade `opentelemetry` and `opentelemetry_sdk` to `v0.22`.
- Upgrade `opentelemetry-http` to `v0.11`.
- Remove `surf-client` feature, since [`opentelemetry-http/surf` has been removed](https://github.com/open-telemetry/opentelemetry-rust/pull/1537).
- Upgrade `opentelemetry-semantic-conventions` to `v0.14`.
- Change `opentelemetry_application_insights::attrs::*` from `opentelemetry::Key` to `&str`. This matches the [change in `opentelemetry-semantic-conventions`](https://github.com/open-telemetry/opentelemetry-rust/issues/1320).
- Upgrade `sysinfo` to `v0.30`.

## [0.29.0] - 2023-11-18

- Upgrade to `v0.21.0` of `opentelemetry` and `opentelemetry_sdk`.
- Upgrade to `v0.10.0` of `opentelemetry-http`.
- Upgrade to `v0.13.0` of `opentelemetry-semantic-conventions`.
- Improve live metrics performance ([#64](https://github.com/frigus02/opentelemetry-application-insights/pull/64)).

## [0.28.0] - 2023-10-22

- Support for [Live Metrics](https://learn.microsoft.com/en-us/azure/azure-monitor/app/live-stream).

## [0.27.0] - 2023-08-06

- Support configuration via connection string (see [#54](https://github.com/frigus02/opentelemetry-application-insights/issues/54)).
  - Configuration via instrumentation key is now deprecated.

## [0.26.0] - 2023-08-05

- Upgrade to `v0.20.0` of `opentelemetry`.
- Upgrade to `v0.12.0` of `opentelemetry-semantic-conventions`.
  - This removes and deprecates some attributes. All removed/deprecated attributes continue to work. Consider migrating to the new attributes in the future. See "Deprecated attributes" in opentelemetry-application-insights documentation for suitable replacements.

## [0.25.0] - 2023-03-26

- Upgrade to `v0.19.0` of `opentelemetry`.
- Upgrade to `v0.11.0` of `opentelemetry-semantic-conventions`.
  - This removes `trace::HTTP_HOST` and `trace::NET_PEER_IP`. The keys `"http.host"` and `"net.peer.ip"` continue to work with this crate. Consider migrating to `"host.request.header.host"` and `"net.peer.name"` (`trace::NET_PEER_NAME`) / `"net.sock.peer.addr"` (`trace::NET_SOCK_PEER_ADDR`).

## [0.24.0] - 2023-03-11

- Map `level` attribute (set by [`tracing::Level`](https://docs.rs/tracing/0.1.37/tracing/struct.Level.html)) to Application Insights severity level (see [#57](https://github.com/frigus02/opentelemetry-application-insights/pull/57)).

## [0.23.0] - 2022-11-12

- Support sending `customEvents` (see [#53](https://github.com/frigus02/opentelemetry-application-insights/issues/53)).

## [0.22.0] - 2022-09-17

- Upgrade to `v0.18.0` of `opentelemetry`.

## [0.21.0] - 2022-07-02

- New feature `reqwest-client-vendored-tls`, which makes it easy to use `reqwest` with its `native-tls-vendored` feature.

## [0.20.0] - 2022-03-20

- Use gzip compression (`Content-Encoding: gzip`) for `POST /v2/track` HTTP request.

## [0.19.0] - 2022-01-23

- Upgrade to `v0.17.0` of `opentelemetry`.

## [0.18.0] - 2021-09-12

- Add support for metrics.
- Take span resource into account for tags. Before a `service.name` in the resource would not populate the Cloud role name tag. Now it does.

## [0.17.0] - 2021-08-08

- Upgrade to `v0.16.0` of `opentelemetry`.

## [0.16.0] - 2021-06-17

- Upgrade to `v0.15.0` of `opentelemetry`.

## [0.15.0] - 2021-05-24

### Changed

- Upgrade to `v0.14.0` of `opentelemetry`. Thanks to [@notheotherben](https://github.com/notheotherben).

- Use HttpClient trait from `opentelemetry-http`.

## [0.14.0] - 2021-05-03

### Added

- Add `with_service_name` function to pipeline builder, which makes it easier to specify a service name (translated to Cloud Role Name in Application Insights). Thanks to [@isobelhooper](https://github.com/isobelhooper) and [@johnchildren](https://github.com/johnchildren).

## [0.13.0] - 2021-03-25

### Changed

- Upgrade to `v0.13.0` of `opentelemetry`.

  The choice of simple/batch span processor as well as the async runtime now needs to be made in Rust code:

  - If you previously used `.install()` with the `reqwest::blocking::Client`, you should now use `.install_simple()`.
  - If you previously used `.install()` with the `reqwest::Client` and the Tokio runtime, you should now use `.install_batch(opentelemetry::runtime::Tokio)` as well as enable to **opentelemetry/rt-tokio** feature.
  - If you previously used `.install()` with the `surf::Client` and the async-std runtime, you should now use `.install_batch(opentelemetry::runtime::AsyncStd)` as well as enable to **opentelemetry/rt-async-std** feature.

## [0.12.0] - 2021-03-19

### Added

- New feature flags for using `rustls` with the `reqwest` clients instead of `nativetls`

## [0.11.0] - 2021-01-22

### Changed

- Upgrade to `v0.12.0` of `opentelemetry`.

## [0.10.0] - 2020-12-29

### Changed

- Upgrade to `v0.11.0` of `opentelemetry`.

## [0.9.0] - 2020-12-23

### Added

- Constants for span attribute keys, which you can use to set any of the Application nsights conrext properties (tags).

## [0.8.0] - 2020-12-08

### Added

- Support for specifying any Application Insights context property (tag) by using span attributes with the internal names of the tags. E.g. the attribute `ai.device.oemName` sets the client device OEM name context. Thanks to [@twitchax](https://github.com/twitchax) for suggesting and implementing this.

## [0.7.1] - 2020-12-03

### Fixed

- Request and remote dependency durations were off by a factor of 10 ([#23](https://github.com/frigus02/opentelemetry-application-insights/issues/23)). Thanks to [@twitchax](https://github.com/twitchax) for finding and fixing the bug.

## [0.7.0] - 2020-12-02

### Added

- Made ingestion endpoint configurable ([#20](https://github.com/frigus02/opentelemetry-application-insights/issues/20)).

## [0.6.0] - 2020-11-10

### Changed

- Upgrade to `v0.10.0` of `opentelemetry`.

## [0.5.0] - 2020-10-18

### Added

- Added `PipelineBuilder`, a more ergonomic way to configure OpenTelemetry with the Application Insights span exporter.
- Added support for semantic conventions for exceptions. Events with the name "exception" are now converted into Exception telemetry.

### Changed

- Upgrade to `v0.9.0` of `opentelemetry`. This makes span exports use async/await. In order to support different runtimes, you now need to specify a compatible HTTP client.

## [0.4.0] - 2020-08-14

### Changed

- Upgrade to `v0.8.0` of `opentelemetry`.

- Removed `.with_application_version` function on exporter. Please use the `service.version` resource attribute instead. See [semantic conventions](https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/resource/semantic_conventions#service) for about the attribute.

  ```rust
  sdk::Provider::builder()
      .with_config(sdk::Config {
          resource: Arc::new(sdk::Resource::new(vec![
              KeyValue::new("service.version", concat!("semver:", env!("CARGO_PKG_VERSION"))),
          ])),
          ..sdk::Config::default()
      })
      .build();
  ```

- Internal SDK Version is now filled from the resource attributes `telemetry.sdk.name` and `telemetry.sdk.version`.

## [0.3.0] - 2020-08-09

### Added

- Automatically truncate any values that are too long for Application Insight.
- Request Url is now additionally constructed from `http.scheme`, `http.host` and `http.target` if all three are available.
- Request Source is now filled from`http.client_ip` or `net.peer.ip`.
- Dependency Target now includes the port from `net.peer.port` and falls back to `net.peer.ip` if the name is not available.
- Dependency Type now additionally looks into `rpc.system`.

### Changed

- Update attribute mapping based on new semantic conventions. The Dependency Target is now filled from `db.name` (before `db.instance`) and the Dependency Type is now filled from `db.system` (before `db.type`). Thanks [@johnchildren](https://github.com/johnchildren).
- Cloud Role is now filled from the resource attributes `service.namespace` and `service.name` (before it was autimatically filled from the current process executable name). This follows the OpenTelemetry specification and gives users of the exporter more flexibility.
- Cloud Role Instance is now filled from the resource attribute `service.instance.id` (before it was filled automatically from the machine's hostname). This follows the OpenTelemetry specification and gives users of the exporter more flexibility.

## [0.2.0] - 2020-08-05

### Changed

- Upgrade to `v0.7.0` of `opentelemetry`.

## [0.1.2] - 2020-07-25

### Added

- Send span resource attributes as part of request/dependency/trace custom properties. Thanks [@tot0](https://github.com/tot0).

## [0.1.1] - 2020-06-23

### Added

- Populate the cloud role tag from the current process name.

### Changed

- Aligned attribute mapping with Azure Monitor exporter for [Python](https://github.com/microsoft/opentelemetry-azure-monitor-python) and [JavaScript](https://github.com/microsoft/opentelemetry-azure-monitor-js). Most notably, `INTERNAL` spans now create a Dependency.
- Populate cloud role instance tag from machine hostname instead of using the `net.host.name` attribute.

### Fixed

- Support events with empty messages. They now get the default message `"<no message>"`.

## 0.1.0 - 2020-06-21

### Added

- First release.

[unreleased]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.38.0...HEAD
[0.38.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.37.0...0.38.0
[0.37.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.36.0...0.37.0
[0.36.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.35.0...0.36.0
[0.35.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.34.0...0.35.0
[0.34.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.33.0...0.34.0
[0.33.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.32.0...0.33.0
[0.32.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.31.0...0.32.0
[0.31.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.30.0...0.31.0
[0.30.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.29.0...0.30.0
[0.29.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.28.0...0.29.0
[0.28.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.27.0...0.28.0
[0.27.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.26.0...0.27.0
[0.26.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.25.0...0.26.0
[0.25.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.24.0...0.25.0
[0.24.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.23.0...0.24.0
[0.23.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.22.0...0.23.0
[0.22.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.21.0...0.22.0
[0.21.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.20.0...0.21.0
[0.20.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.19.0...0.20.0
[0.19.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.18.0...0.19.0
[0.18.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.17.0...0.18.0
[0.17.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.16.0...0.17.0
[0.16.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.15.0...0.16.0
[0.15.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.14.0...0.15.0
[0.14.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.13.0...0.14.0
[0.13.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.12.0...0.13.0
[0.12.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.11.0...0.12.0
[0.11.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.10.0...0.11.0
[0.10.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.7.1...0.8.0
[0.7.1]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.7.0...0.7.1
[0.7.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.6.0...0.7.0
[0.6.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.5.0...0.6.0
[0.5.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.2...0.2.0
[0.1.2]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.0...0.1.1
