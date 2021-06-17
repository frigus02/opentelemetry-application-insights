# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.16.0...HEAD
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
