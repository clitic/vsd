# vsd Changelog (DD/MM/YYYY)

## 0.1.3 (dev)

Features:

- Better variant stream selection and display order.
- Using chrome request headers when using *--collect* flag to send the request.

Changes:

- Pre fetching playlists when using *--collect* flag.

Bug Fixes:

- `.vtt` -> `.srt` conversion ffmpeg command correction.
- No website scraping when extension is `.m3u`.
- No panic when alternative streams are 0.

## 0.1.2 (09/07/2022)

Features:

- New `--build` flag.
- `.srt` subtitles collection with `--collect` flag.

Bug Fixes:

- Not intercepting requests before navigating to website when using `--capture` and `--collect` flags.

## 0.1.0 (22/06/2022)

Features:

- Initial release
