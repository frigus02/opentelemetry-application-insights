# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Aligned attribute mapping with Azure Monitor exporter for [Python](https://github.com/microsoft/opentelemetry-azure-monitor-python) and [JavaScript](https://github.com/microsoft/opentelemetry-azure-monitor-js). Most notably, `INTERNAL` spans now create a Dependency.

### Fixed

- Support events with empty messages. They now get the default message `"<no message>`.

## 0.1.0 - 2020-06-22

### Added

- First release.

[unreleased]: https://github.com/frigus02/opentelemetry-application-insights/compare/0.1.0...HEAD
