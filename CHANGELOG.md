# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support for decrypting SAMPLE-AES (com.apple.streamingkeydelivery)

### Changed

- Do not use space character when saving file, instead use `vsd_*` prefix.

### Fixed

- Relative url build using baseurl for local `.mpd` files.
- Segmentation fault when using threads more than 1.
- Subtitles saved as `.txt` but ffmpeg command uses `.vtt`.

## [0.2.0] - 2022-10-08

### Added

- *DASH* support with decryption and subtitles.
- Subcommands instead of a single command where *save* is the main subcommand.
- New singular progress bar for complete download progress.
- Better variant stream selection and display order.
- Improved support for playlists using byte range.
- Improved *capture* and *collect* subcommands.
  - Using response received url when using *capture* subcommand.
  - Using chrome response for fetching playlists when using *collect* subcommand.

### Changed

- Default command is split into *save*, *capture* and *collect* subcommands.
- Resume support is removed for now.

### Fixed

- `.vtt` -> `.srt` conversion ffmpeg command correction.
- No website scraping when extension is `.m3u`.

## [0.1.2] - 2022-07-09

### Added

- New `--build` flag.
- `.srt` subtitles collection with `--collect` flag.

### Fixed

- Not intercepting requests before navigating to website when using `--capture` and `--collect` flags.

## [0.1.0] - 2022-06-22

[Unreleased]: https://github.com/clitic/vsd/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/clitic/vsd/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/clitic/vsd/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/clitic/vsd/releases/tag/v0.1.0
