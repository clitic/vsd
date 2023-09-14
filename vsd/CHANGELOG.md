# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `save`
  - Handle `--output` flag correctly. ([#21](https://github.com/clitic/vsd/issues/21))

## [0.3.0] - 2023-08-18

### Added

- `--color` flag to control when to output colored text.
- `capture`
  - `--cookies`, `--directory`, `--save`, `--extensions` and `--resource-types` flags.
- `save`
  - Support for HLS `#EXT-X-MAP` tag.
  - Support for multi period DASH manifests.
  - Support for downloading playlists with single segment.
  - Support for parsing pssh box from initialization and displaying all key ids.
  - Support for browser cookies i.e. with `--cookies` flag.
  - Support for socks proxy.
  - `min` / `lowest` quality option for `--quality` flag.
  - `--skip-prompts`, `--all-keys`, `--no-decrypt` and `--no-certificate-checks` flags.

### Changed

- `capture`
  - `collect` sub-command is merged with `capture` sub-command.
  - `--build` flag is removed from `collect` sub-command.
- `collect` sub-command is removed.
- `decrypt` sub-command is removed.
- `extract`
  - `input` now only accepts single file.
  - `--format` flag is replaced with `--codec` flag.
- `merge`
  - `--ffmpeg` flag is replaced with `--type` flag.
- `save`
  - Video stream selection prompt is replaced with multi select prompt.
    Also, `--alternative` flag is removed and merged in this prompt. 
  - Use more accurate units (KiB, MiB, ..) to show download progress.
    Also, spinner is removed from progress bar.
  - `--cookies` flag is renamed as `--set-cookie`.
  - `--baseurl` flag is renamed as `--base-url`.
  - `--proxy-address` flag is renamed as `--proxy`.
  - `--quality` flag now also matches height if that specific resolution is not found.

### Fixed

- `capture`
  - Handle `CTRL+C` signal correctly.
- `save`
  - Some program panics when auto selecting streams using `--quality` flag.
  - `--directory` and `--header` flag implementation.
  - Unknown errors while extracting `stpp` and `application/ttml+xml` streams.
  - Use HLS `#EXT-X-KEY` tag more correctly.
  - DASH stream parsing logic.

## [0.2.5] - 2023-01-09

### Changed

- Do not use space character when saving file, instead use `vsd_*` prefix.
- `capture` and `collect` sub-commands are kept under optional cargo feature (`chrome`) but this feature is enabled by default.

### Fixed

- Relative url build using baseurl for local `.mpd` files.
- Segmentation fault when using threads more than 1.
- Subtitles saved as `.txt` but ffmpeg command uses `.vtt`.
- Match playlist kid(s) correctly with `--key` flag.

## [0.2.0] - 2022-10-08

### Added

- *DASH* support with decryption and subtitles.
- Sub-commands instead of a single command where *save* is the main sub-command.
- New singular progress bar for complete download progress.
- Better variant stream selection and display order.
- Improved support for playlists using byte range.
- Improved *capture* and *collect* sub-commands.
  - Using response received url when using *capture* sub-command.
  - Using chrome response for fetching playlists when using *collect* sub-command.

### Changed

- Default command is split into *save*, *capture* and *collect* sub-commands.
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

[Unreleased]: https://github.com/clitic/vsd/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/clitic/vsd/compare/v0.2.5...v0.3.0
[0.2.5]: https://github.com/clitic/vsd/compare/v0.2.0...v0.2.5
[0.2.0]: https://github.com/clitic/vsd/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/clitic/vsd/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/clitic/vsd/releases/tag/v0.1.0
