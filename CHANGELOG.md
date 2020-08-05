# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.2.0...HEAD
[0.2.0]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.2...0.2.0
[0.1.2]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.0...0.1.1
