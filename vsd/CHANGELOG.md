# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2025-06-13

### Added

- `capture`
  - Support for downloading session cookies.
  - Support for proxy server.
- `save`
  - More robust subtitles codec detection.
  - New and improved automation support.
  - Support for HLS `SAMPLE-AES` stream decrytion.

### Changed

- `save`
  - Now by default vsd proceeds with default stream selections. The old behaviour can still be used using `-i, --interactive` and `--interactive-raw` flags.
  - One unified progress bar tracking the entire download.
  - Removed `--no-query-pass` flag.
  - Removed support for custom hls key.
  - Renamed `--retry-count` to `--retries` flag.
  - Unknown codec subtitles can now ne downloaded.
 
### Fixed

- `save`
  - Mux commands for solo streams.
  - Query support.

## [0.3.3] - 2025-03-14

### Added

- `save`
  - `--no-query-pass` flag.
  - `--query` flag.
- Optimized threads management.

### Changed

- `--key` flag now has a different syntax.
- Query parameters are passed on by default now. This behaviour can be changed using `--no-query-pass` flag.

### Fixed

- Ffmpeg can now be fetched from working directory. ([#42](https://github.com/clitic/vsd/issues/42))
- Passing query parameters for DASH playlists. ([#36](https://github.com/clitic/vsd/issues/36))

## [0.3.2] - 2024-06-23

### Changed

- Removed self update checker.
- Website scraper now only parses qouted links.

### Fixed

- `--cookies` flag and it's implementation.
- Amalgated links found through website scraper.
- Detect missing ffmpeg binary when it is required.

## [0.3.1] - 2024-06-22

### Added

- `save`
  - `--no-merge` flag. ([#17](https://github.com/clitic/vsd/issues/17), [#20](https://github.com/clitic/vsd/issues/20))
  - `--parse` flag.

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

[Unreleased]: https://github.com/clitic/vsd/compare/0.4.0...HEAD
[0.4.0]: https://github.com/clitic/vsd/compare/0.3.3...vsd-0.4.0
[0.3.3]: https://github.com/clitic/vsd/compare/0.3.2...0.3.3
[0.3.2]: https://github.com/clitic/vsd/compare/0.3.1...0.3.2
[0.3.1]: https://github.com/clitic/vsd/compare/v0.3.0...0.3.1
[0.3.0]: https://github.com/clitic/vsd/compare/v0.2.5...v0.3.0
[0.2.5]: https://github.com/clitic/vsd/compare/v0.2.0...v0.2.5
[0.2.0]: https://github.com/clitic/vsd/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/clitic/vsd/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/clitic/vsd/releases/tag/v0.1.0
