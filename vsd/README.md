<h1 align="center">vsd</h1>

<p align="center">
  <a href="https://github.com/clitic/vsd">
    <img src="https://img.shields.io/github/downloads/clitic/vsd/total?logo=github&style=flat-square">
  </a>
  <a href="https://crates.io/crates/vsd">
    <img src="https://img.shields.io/crates/d/vsd?logo=rust&style=flat-square">
  </a>
  <a href="https://crates.io/crates/vsd">
    <img src="https://img.shields.io/crates/v/vsd?style=flat-square">
  </a>
  <a href="https://github.com/clitic/vsd">
    <img src="https://img.shields.io/github/actions/workflow/status/clitic/vsd/build.yml?logo=github&style=flat-square">
  </a>
  <a href="https://github.com/clitic/vsd#license">
    <img src="https://img.shields.io/crates/l/vsd?style=flat-square">
  </a>
  <a href="https://github.com/clitic/vsd">
    <img src="https://img.shields.io/github/repo-size/clitic/vsd?logo=github&style=flat-square">
  </a>
  <a href="https://colab.research.google.com/github/clitic/vsd/blob/main/vsd/vsd-on-colab.ipynb">
    <img src="https://img.shields.io/badge/Open%20In%20Colab-F9AB00?logo=googlecolab&color=525252&style=flat-square">
  </a>
</p>

**V**ideo **S**tream **D**ownloader is a powerful command-line utility that enables users to download video content streamed over HTTP from websites. It supports both [DASH (Dynamic Adaptive Streaming over HTTP)](https://en.wikipedia.org/wiki/Dynamic_Adaptive_Streaming_over_HTTP) using `.mpd` manifest files and [HLS (HTTP Live Streaming)](https://en.wikipedia.org/wiki/HTTP_Live_Streaming) using `.m3u8` playlists. The tool is designed to handle adaptive bitrate streams, fetch individual video and audio segments, and optionally mux them into a single playable file, making it ideal for offline viewing, archival, or analysis of online video content.

<p align="center">
  <img src="https://github.com/clitic/vsd/blob/main/vsd/images/showcase.gif" width="700">
</p>

<div align="center">
    <a href="https://www.buymeacoffee.com/clitic" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="41px"></a>
</div>

## Table of Contents

- [Features](#features)
- [Installation](#installation)
  - [Dependencies](#dependencies)
  - [Pre-built Binaries](#pre-built-binaries)
  - [Install via Cargo](#install-via-cargo)
  - [Additional Resources](#additional-resources)
- [Usage](#usage)
- [Running on Android](#running-on-android)
- [Alternatives](#alternatives)
- [License](#license)

## Features

- [x] Captures network requests and lists playlist and subtitle files from websites.
- [x] Compatible with both DASH and HLS playlists.
- [x] Enables multi-threaded downloading for faster performance.
- [x] Muxing streams to single video container using ffmpeg.
- [x] Offers robust automation support.
- [x] One unified progress bar tracking the entire download, with real-time file size updates.
- [x] Supports decryption for `AES-128`, `SAMPLE-AES`, `CENC`, `CBCS`, `CENS` and `CBC1`.
- [ ] Live stream downloading (not currently planned).

<a href="#Help">See More</a>

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

[![Packaging status](https://repology.org/badge/vertical-allrepos/vsd.svg)](https://repology.org/project/vsd/versions)

### Install via Cargo

You can also install vsd using cargo.

```bash
$ cargo install vsd
```

### Additional Resources

- [Build Instructions](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md)
- [Changelog](https://github.com/clitic/vsd/blob/main/vsd/CHANGELOG.md)

## Usage

Below are some example commands. For additional usage details, see [CLI.md](https://github.com/clitic/vsd/blob/main/vsd/CLI.md).

- Capture playlists and subtitles from a website.

```bash
$ vsd capture <url> --save-cookies
```

> The saved cookies can be used as `--cookies cookies.json` with `save` sub-command later on.

- Download playlists. ([test streams](https://test-streams.mux.dev))

```bash
$ vsd save <url> -o video.mp4
```

> Use `-i, --interactive` flag to open an interactive session.

- Download encrypted playlists. ([drm test vectors](https://github.com/Axinom/public-test-vectors))

```bash
$ vsd save https://bitmovin-a.akamaihd.net/content/art-of-motion_drm/mpds/11331.mpd \
    --keys "eb676abbcb345e96bbcf616630f1a3da:100b6c20940f779a4589152b57d2dacb" \
    -o video.mp4
```

- List and select specific streams from a playlist.

```bash
$ vsd save <url> --list-streams
$ vsd save <url> --select-streams "v=1,2:a=3" -o video.mp4
```

- Prefer some specific languages when downloading audio/subtitles.

```bash
$ vsd save <url> --select-streams "a=en,fr:s=en,fr" -o video.mp4
```

- Use as a playlist parser. ([json schema](https://github.com/clitic/vsd/blob/main/vsd/src/playlist.rs))

```bash
$ vsd save <url> --parse > parsed-playlist.json
```

## Running on Android

1. Install the [Termux](https://termux.com) app on your device, then enable storage permissions manually from its settings page. After that, run the following commands in the terminal.

```bash
$ pkg update
$ pkg upgrade
$ pkg install ffmpeg
$ ln -s /storage/emulated/0/Download Download
```

2. Install [vsd on termux](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md#android-on-termux). Currently, only *arm64-v8a* binaries pre-builts are available which can be installed using the following command.

```bash
curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-linux-android.tar.xz | tar xJC $PREFIX/bin
```

3. Use third party browsers like [Kiwi Browser](https://github.com/kiwibrowser/src.next) (*developer tools*) paired with [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc) extension or [Via Browser](https://play.google.com/store/apps/details?id=mark.via.gp) (*tools > resource sniffer*) to find playlists within websites.

4. Now you can run vsd as usual. The streams would be directly downloaded in your android downloads folder.  

```bash
$ cd Download
$ vsd save <url> -o video.mp4
```

## Alternatives

List of alternatives to vsd:

1. [N_m3u8DL-RE](https://github.com/nilaoda/N_m3u8DL-RE) is the best alternative to vsd. It also supports live playlists, which vsd does not. However, it lacks features like *capture* functionality.
2. [yt-dlp](https://github.com/yt-dlp/yt-dlp) is excellent for downloading various playlists, but its main drawback is limited support for decryption.
3. [dash-mpd-cli](https://github.com/emarsden/dash-mpd-cli) is a highly effective tool for downloading DASH playlists. In fact, much of vsd’s internal logic for parsing and downloading DASH content is based on this tool.
4. [ffmpeg](https://ffmpeg.org) supports direct encoding of playlists.
5. Both [streamlink](https://github.com/streamlink/streamlink) and [vlc](https://www.videolan.org/vlc) allow direct streaming of playlists.
## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
