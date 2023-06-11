# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.2] - 2023-06-11

### Added

- Support for linking against prebuilt ap4 library.

## [0.4.1] - 2023-05-10

### Changed

- Upgraded `bento4-src` v0.1.0 -> v0.1.1

## [0.4.0] - 2023-05-10

### Changed

- Use `bento4-src` crate while building this crate. 
- Removed `mp4split` function.

## [0.3.1] - 2023-01-07

### Changed

- Applied thread safety according to Bento4 issue [#783](https://github.com/axiomatic-systems/Bento4/issues/783).

## [0.3.0] - 2023-01-06

### Added

- Thread safety (by using a crate lock).

### Fixed

- Segmentation fault when using multiple threads.

## [0.2.1] - 2022-09-18

### Added

- New `mp4split` function.

## [0.2.0] - 2022-09-13

### Added

- New `mp4split` function.

## [0.1.1] - 2022-08-31

### Fixed

- Include `build.rs`

## [0.1.0] - 2022-08-31

[Unreleased]: https://github.com/clitic/vsd/compare/mp4decrypt-v0.4.1...HEAD
[0.4.1]: https://github.com/clitic/vsd/compare/mp4decrypt-v0.4.0...mp4decrypt-v0.4.1
[0.4.0]: https://github.com/clitic/mp4decrypt/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/clitic/mp4decrypt/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/clitic/mp4decrypt/compare/5759e24...v0.3.0
[0.2.1]: https://github.com/clitic/mp4decrypt/compare/56680c2...5759e24
[0.2.0]: https://github.com/clitic/mp4decrypt/compare/843bb3d...56680c2
[0.1.1]: https://github.com/clitic/mp4decrypt/compare/d2490fc...843bb3d
[0.1.0]: https://github.com/clitic/mp4decrypt/compare/3c00224...d2490fc
