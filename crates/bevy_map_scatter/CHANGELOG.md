# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-10-21

### Changed

- Make `serde` and `ron` optional dependencies behind features "serde" and "ron".
- Default features include both `serde` and `ron` to keep `.scatter` asset loading working out of the box.

## [0.2.0] - 2025-10-06

### Added

- Initial release of `bevy_map_scatter` core crate.
