<h1 align="center">vsd</h1>

[![Github Downloads](https://img.shields.io/github/downloads/clitic/vsd/total?logo=github&style=flat-square)](https://github.com/clitic/vsd/releases)
[![Crate Downloads](https://img.shields.io/crates/d/vsd?logo=rust&style=flat-square)](https://crates.io/crates/vsd)
[![Crate Version](https://img.shields.io/crates/v/vsd?logo=rust&style=flat-square)](https://crates.io/crates/vsd)
[![Build Status](https://img.shields.io/github/actions/workflow/status/clitic/vsd/build.yml?logo=github&style=flat-square)](https://github.com/clitic/vsd/actions)
[![Docs Status](https://img.shields.io/docsrs/vsd?logo=docsdotrs&style=flat-square)](https://docs.rs/vsd)
[![Crate License](https://img.shields.io/crates/l/vsd?style=flat-square)](https://crates.io/crates/vsd)
[![Repo Size](https://img.shields.io/github/repo-size/clitic/vsd?logo=github&style=flat-square)](https://github.com/clitic/vsd)

**V**ideo **S**tream **D**ownloader is a command-line utility and Rust library for downloading video streams from [DASH](https://en.wikipedia.org/wiki/Dynamic_Adaptive_Streaming_over_HTTP) `.mpd` manifests and [HLS](https://en.wikipedia.org/wiki/HTTP_Live_Streaming) `.m3u8` playlists. It handles multi-threaded segment fetching, DRM decryption, subtitle extraction, and ffmpeg-based muxing.

<div align="center">
  <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/images/showcase.gif" width="700px">
</div>

## Features

- [x] **DASH & HLS Support**: Supports both DASH `.mpd` manifests and HLS `.m3u8` playlists.
- [x] **DRM Support**: Decrypt protected content using keys acquired from Widevine and PlayReady license servers.
- [x] **Subtitles Extraction**: Extract subtitle tracks from fragmented mp4 streams.
- [x] **Multi-threaded Downloads**: Fetch media segments concurrently with customizable thread counts to maximize bandwidth.
- [x] **Rust Library Support**: Integrate `vsd` directly into your rust projects as a library.
- [ ] **Live Stream Downloading**: Live stream downloading is not yet supported.

## [Installation](https://clitic.github.io/vsd/installation)
  
### Dependencies

- [ffmpeg](https://www.ffmpeg.org/download.html) (optional, *recommended*) required for transmuxing streams.
- [chrome](https://www.google.com/chrome) / [chromium](https://www.chromium.org/getting-involved/download-chromium/) (optional) needed only for the [capture](https://clitic.github.io/vsd/cli/#vsd-capture) sub-command. 

### Pre-built Binaries

Visit the [releases page](https://github.com/clitic/vsd/releases) for pre-built binaries or grab the [latest CI builds](https://nightly.link/clitic/vsd/workflows/build/main). Extract the binary and add its path to your system's `PATH`.

[![Packaging Status](https://repology.org/badge/vertical-allrepos/vsd.svg)](https://repology.org/project/vsd/versions)

### Additional Resources

- [Build Instructions](https://clitic.github.io/vsd/build)
- [Changelog](https://clitic.github.io/vsd/CHANGELOG)

## Usage

For detailed usage examples, see the [usage](https://clitic.github.io/vsd/usage) page. For a complete list of commands and options, see the [CLI reference](https://clitic.github.io/vsd/cli).

The main entry point is the [save](https://clitic.github.io/vsd/cli/#vsd-save) sub-command, which downloads streams from a DASH or HLS playlist. When an output path is provided, it can also mux them into a single file using ffmpeg.

```bash
vsd save "https://media.axprod.net/TestVectors/Hls/not_protected_hls_1080p_h264/manifest.m3u8" -o output.mp4
```

<details>
<summary>Click to see output</summary>

```
Stream [vid] 1920x1080 | 4140k | avc1.640028… |   ? fps
Stream [vid]  1280x720 | 2982k | avc1.64001f… |   ? fps
Stream [vid]  1024x576 | 1799k | avc1.64001f… |   ? fps
Stream [vid]   640x360 | 1272k | avc1.64001e… |   ? fps
Stream [vid]   512x288 |  738k | avc1.640015… |   ? fps
Stream [vid]   384x216 |  617k | avc1.64000d… |   ? fps
Stream [aud]        en |     ? |            ? |   2 ch
Stream [aud]    en-low |     ? |            ? |   2 ch
Stream [aud]   en-high |     ? |            ? |   2 ch
Stream [sub]        fr |    ?k |            ? |
Stream [sub]        en |    ?k |            ? |
Stream [sub]        de |    ?k |            ? |
DownLd [vid] 1920x1080 4140k avc1.640028… ?fps
Saving [vid] vsd-vid-f3f2d26
[#(1/3) 258.3MiB/~258.3MiB(100%) PT:184/184 DL:1.2MiB ETA:0s]
Concat [vid] vsd-vid-f3f2d26.ts
DownLd [aud] en ? ? 2ch
Saving [aud] vsd-aud-8053bac
[#(2/3) 16.8MiB/~16.8MiB(100%) PT:184/184 DL:1.4MiB ETA:0s]
Concat [aud] vsd-aud-8053bac.ts
DownLd [sub] fr ?k ?
Saving [sub] vsd-sub-c7fc10a.vtt
[#(3/3) 15.7KiB/~15.7KiB(100%) PT:184/184 DL:2.0KiB ETA:0s]
Muxing [exe] ffmpeg -hide_banner -y -i vsd-vid-f3f2d26.ts -i vsd-aud-8053bac.ts -i vsd-sub-c7fc10a.vtt -map 0 -map 1 -map 2 -metadata:s:a:0 language=en -metadata:s:s:0 language=fr -disposition:a:0 default -disposition:s:0 default -c:v copy -c:a copy -c:s mov_text output.mp4
```

</details>

## Library

Add this to your Cargo.toml file.

```toml
[dependencies]
vsd = { version = "0.5", default-features = false, features = ["rustls-tls"]}
```

Or add from command line.

```bash
cargo add vsd --no-default-features --features rustls-tls
```

See [docs](https://docs.rs/vsd) and [examples](https://github.com/clitic/vsd/tree/main/vsd/examples) to 
know how to use it.

## vsd vs N_m3u8DL-RE

[N_m3u8DL-RE](https://github.com/nilaoda/N_m3u8DL-RE) is another popular streaming downloader. This section helps you decide which tool fits your needs.

| Feature | vsd | N_m3u8DL-RE |
|---|---|---|
| DASH | ✅ | ✅ |
| HLS | ✅ | ✅ |
| MSS | ❌ | ✅ |
| Live Stream Recording | ❌ | ✅ |
| Built-in Decryption | ✅ | ✅ |
| DRM License Acquisition | ✅ | ❌ |
| Concurrent Downloads | ❌ | ✅ |
| Browser Capture | ✅ | ❌ |
| Library Support | ✅ | ❌ |

## Donate

This project is developed and maintained in my free time. Donations help cover development time, testing, and future improvements. If this tool saved you time or helped your workflow, consider supporting it.

<div align="center">
  <a href="mailto:clitic21@gmail.com" target="_blank" style="text-decoration: none;">
    <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/assets/contact.svg" alt="Contact Me" height="42px">
  </a>
  <a href="https://ko-fi.com/clitic" target="_blank" style="text-decoration: none;">
    <img src="https://storage.ko-fi.com/cdn/kofi5.png?v=6" alt="Buy Me a Coffee at ko-fi.com" height="40px" />
  </a>
  <a href="https://www.buymeacoffee.com/clitic" target="_blank" style="text-decoration: none;">
    <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/assets/bmc.svg" alt="Buy Me A Coffee" height="40px">
  </a>
  <a href="https://paypal.me/clitic" target="_blank" style="text-decoration: none;">
    <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/assets/paypal.svg" alt="PayPal" height="40px">
  </a>
</div>

## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
