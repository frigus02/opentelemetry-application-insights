# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.4.0...HEAD
[0.4.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.2...0.2.0
[0.1.2]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.0...0.1.1
