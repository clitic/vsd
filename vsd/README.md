<h1 align="center">vsd</h1>

[![Github Downloads](https://img.shields.io/github/downloads/clitic/vsd/total?logo=github&style=flat-square)](https://github.com/clitic/vsd/releases)
[![Crate Downloads](https://img.shields.io/crates/d/vsd?logo=rust&style=flat-square)](https://crates.io/crates/vsd)
[![Crate Version](https://img.shields.io/crates/v/vsd?style=flat-square)](https://crates.io/crates/vsd)
[![Build Status](https://img.shields.io/github/actions/workflow/status/clitic/vsd/build.yml?logo=github&style=flat-square)](https://github.com/clitic/vsd/actions)
[![Crate License](https://img.shields.io/crates/l/vsd?style=flat-square)](https://crates.io/crates/vsd)
[![Repo Size](https://img.shields.io/github/repo-size/clitic/vsd?logo=github&style=flat-square)](https://github.com/clitic/vsd)
[![Open In Colab](https://img.shields.io/badge/Open%20In%20Colab-F9AB00?logo=googlecolab&color=525252&style=flat-square)](https://colab.research.google.com/github/clitic/vsd/blob/main/vsd/vsd-on-colab.ipynb)

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

## Installation
  
### Dependencies

- [ffmpeg](https://www.ffmpeg.org/download.html) (optional, *recommended*) required for transmuxing and transcoding streams.
- [chrome](https://www.google.com/chrome) / [chromium](https://www.chromium.org/getting-involved/download-chromium/) (optional) needed only for the capture sub-command. 

### Pre-built Binaries

Visit the [releases page](https://github.com/clitic/vsd/releases) for pre-built binaries or grab the [latest CI builds](https://nightly.link/clitic/vsd/workflows/build/main).
Download and extract the archive, then copy the vsd binary to a directory of your choice.
Finally, add that directory to your system's `PATH` environment variable.

| Host                | Architecture | Download                                                                                                     |
|---------------------|--------------|--------------------------------------------------------------------------------------------------------------|
| Android 7+ (Termux) | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-linux-android.tar.xz)      |
| Linux               | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-unknown-linux-musl.tar.xz) |
| MacOS               | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-apple-darwin.tar.xz)       |
| Windows             | aarch64      | [.zip](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-pc-windows-msvc.zip)          |
| Linux               | x86_64       | [.tar.xz](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-x86_64-unknown-linux-musl.tar.xz)  |
| MacOS               | x86_64       | [.tar.xz](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-x86_64-apple-darwin.tar.xz)        |
| Windows             | x86_64       | [.zip](https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-x86_64-pc-windows-msvc.zip)           |

[![Packaging Status](https://repology.org/badge/vertical-allrepos/vsd.svg)](https://repology.org/project/vsd/versions)

### Install via Cargo

You can also install vsd using cargo.

```bash
cargo install vsd
```

### Additional Resources

- [Build Instructions](https://clitic.github.io/vsd/build)
- [Changelog](https://clitic.github.io/vsd/CHANGELOG)

## Usage

Below are some example commands. For additional usage details, see [cli reference](https://clitic.github.io/vsd/cli).

- Capture playlists and subtitles from a website.

    ```bash
    vsd capture <url> --save-cookies
    ```

    > The saved cookies can be used as `--cookies cookies.txt` with `save` sub-command later on.

- Download playlists. ([test streams](https://test-streams.mux.dev))

    ```bash
    vsd save <url> -o video.mp4
    ```

    > Use `-i, --interactive` flag to open an interactive session.

- Download encrypted playlists. ([drm test vectors](https://github.com/Axinom/public-test-vectors))

    ```bash
    vsd save https://bitmovin-a.akamaihd.net/content/art-of-motion_drm/mpds/11331.mpd \
        --keys "eb676abbcb345e96bbcf616630f1a3da:100b6c20940f779a4589152b57d2dacb" \
        -o video.mp4
    ```

- List and select specific streams from a playlist.

    ```bash
    vsd save <url> --list-streams
    vsd save <url> --select-streams "1,2,3" -o video.mp4
    ```

- Prefer some specific languages when downloading audio/subtitles.

    ```bash
    vsd save <url> --select-streams "a=en,fr:s=en,fr" -o video.mp4
    ```

- Use as a playlist parser. ([json schema](https://github.com/clitic/vsd/blob/main/vsd/src/playlist.rs))

    ```bash
    vsd save <url> --parse > parsed-playlist.json
    ```

## Donate

This project is developed and maintained in my free time. Donations help cover development time, testing, and future improvements. If this tool saved you time or helped your workflow, consider supporting it.

<div align="center">
  <a href="mailto:clitic21@gmail.com" target="_blank">
    <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/assets/contact.svg" alt="Contact Me" height="40px">
  </a>
  <a href="https://www.buymeacoffee.com/clitic" target="_blank">
    <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/assets/bmc.svg" alt="Buy Me A Coffee" height="40px">
  </a>
  <a href="https://paypal.me/clitic" target="_blank">
    <img src="https://raw.githubusercontent.com/clitic/vsd/refs/heads/main/docs/assets/paypal.svg" alt="PayPal" height="40px">
  </a>
</div>

## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
