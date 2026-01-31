---
icon: lucide/house
---

# VSD

**V**ideo **S**tream **D**ownloader is a powerful command-line utility that enables users to download video content streamed over HTTP from websites. It supports both [DASH (Dynamic Adaptive Streaming over HTTP)](https://en.wikipedia.org/wiki/Dynamic_Adaptive_Streaming_over_HTTP) using `.mpd` manifest files and [HLS (HTTP Live Streaming)](https://en.wikipedia.org/wiki/HTTP_Live_Streaming) using `.m3u8` playlists. The tool is designed to handle adaptive bitrate streams, fetch individual video and audio segments, and optionally mux them into a single playable file, making it ideal for offline viewing, archival, or analysis of online video content.

<div align="center">
  <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/images/showcase.gif" width="700px">
</div>

## Features

- [x] Captures network requests and lists playlist and subtitle files from websites.
- [x] Compatible with both DASH and HLS playlists.
- [x] Enables multi-threaded downloading for faster performance.
- [x] Muxing streams to single video container using ffmpeg.
- [x] Offers robust automation support.
- [x] One unified progress bar tracking the entire download, with real-time file size updates.
- [x] Supports decryption for `AES-128`, `SAMPLE-AES`, `CENC`, `CENS`, `CBC1` and `CBCS`.
- [ ] Live stream downloading, consider [contributing](https://github.com/clitic/vsd/fork) this feature.
