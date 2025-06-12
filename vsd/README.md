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

## Table of Contents

- [Features](#features)
- [Installation](#installation)
  - [Dependencies](#dependencies)
  - [Pre-built Binaries](#pre-built-binaries)
  - [Install via Cargo](#install-via-cargo)
  - [Additional Resources](#additional-resources)
- [Usage](#usage)
- [Help](#help)
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
| Android 7+ (Termux) | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-aarch64-linux-android.tar.xz)      |
| Linux               | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-aarch64-unknown-linux-musl.tar.xz) |
| MacOS 11.7+         | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-aarch64-apple-darwin.tar.xz)       |
| Windows             | aarch64      | [.zip](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-aarch64-pc-windows-msvc.zip)          |
| Linux               | x86_64       | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-x86_64-unknown-linux-musl.tar.xz)  |
| MacOS 11.7+         | x86_64       | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-x86_64-apple-darwin.tar.xz)        |
| Windows             | x86_64       | [.zip](https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-x86_64-pc-windows-msvc.zip)           |

### Install via Cargo

You can also install vsd using cargo.

```bash
$ cargo install vsd
```

### Additional Resources

- [Build Instructions](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md)
- [Changelog](https://github.com/clitic/vsd/blob/main/vsd/CHANGELOG.md)

## Usage

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
$ vsd save <url> --select-streams "1,2" -o video.mp4
```

- Prefer some specific languages when downloading audio/subtitles.

```bash
$ vsd save <url> --audio-lang "en,fr" --subs-lang "en,fr" -o video.mp4
```

- Use as a playlist parser. ([json schema](https://github.com/clitic/vsd/blob/main/vsd/src/playlist.rs))

```bash
$ vsd save <url> --parse > parsed-playlist.json
```

## Help

```bash
$ vsd --help
```

```
Download video streams served over HTTP from websites, DASH (.mpd) and HLS (.m3u8) playlists.

Usage: vsd.exe [OPTIONS] <COMMAND>

Commands:
  capture  Capture playlists and subtitles from a website
  extract  Extract subtitles from mp4 boxes
  merge    Merge multiple segments to a single file
  save     Download DASH and HLS playlists
  help     Print this message or the help of the given subcommand(s)

Options:
      --color <COLOR>  When to output colored text [default: auto] [possible values: auto, always, never]
  -h, --help           Print help
  -V, --version        Print version
```

```bash
$ vsd save --help
```

```
Download DASH and HLS playlists

Usage: vsd.exe save [OPTIONS] <INPUT>

Arguments:
  <INPUT>  http(s):// | .mpd | .xml | .m3u8

Options:
      --base-url <BASE_URL>    Base url to be used for building absolute url to segment. This flag is usually needed for
                               local input files. By default redirected playlist url is used
  -d, --directory <DIRECTORY>  Change directory path for temporarily downloaded files. By default current working
                               directory is used
  -o, --output <OUTPUT>        Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg. Note
                               that existing files will be overwritten and downloaded streams will be deleted
      --parse                  Parse playlist and returns it in json format. Note that --output flag is ignored when
                               this flag is used
      --color <COLOR>          When to output colored text [default: auto] [possible values: auto, always, never]
  -h, --help                   Print help

Automation Options:
      --all-streams                      Download all streams with --skip-audio, --skip-video and --skip-subs filters
                                         kept in mind
      --audio-lang <AUDIO_LANG>          Preferred languages when multiple audio streams with different languages are
                                         available. Must be in RFC 5646 format (eg. fr or en-AU). If a preference is not
                                         specified and multiple audio streams are present, the first one listed in the
                                         manifest will be downloaded. The values should be seperated by comma
  -i, --interactive                      Prompt for custom streams selection with modern style input prompts. By default
                                         proceed with defaults
      --interactive-raw                  Prompt for custom streams selection with raw style input prompts. By default
                                         proceed with defaults
  -l, --list-streams                     List all the streams present inside the playlist
      --quality <WIDTHxHEIGHT|HEIGHTp>   Automatic selection of some standard resolution video stream with highest
                                         bandwidth stream variant from playlist. If matching resolution of WIDTHxHEIGHT
                                         is not found then only resolution HEIGHT would be considered for selection.
                                         comman values: [lowest, min, 144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k,
                                         1440p, qhd, 4k, 8k, highest, max] [default: highest]
  -s, --select-streams <SELECT_STREAMS>  Select streams to download by their ids obtained by --list-streams flag. It has
                                         the highest priority among the rest of filters. The values should be seperated
                                         by comma
      --skip-audio                       Skip default audio stream selection
      --skip-subs                        Skip default subtitle stream selection
      --skip-video                       Skip default video stream selection
      --subs-lang <SUBS_LANG>            Preferred languages when multiple subtitles streams with different languages
                                         are available. Must be in RFC 5646 format (eg. fr or en-AU). If a preference is
                                         not specified and multiple subtitles streams are present, the first one listed
                                         in the manifest will be downloaded. The values should be seperated by comma

Client Options:
      --cookies <COOKIES>              Fill request client with some existing cookies value. Cookies value can be same
                                       as document.cookie or in json format same as puppeteer
      --header <KEY> <VALUE>           Custom headers for requests. This option can be used multiple times
      --no-certificate-checks          Skip checking and validation of site certificates
      --proxy <PROXY>                  Set http(s) / socks proxy address for requests
      --query <QUERY>                  Set query parameters for requests
      --set-cookie <SET_COOKIE> <URL>  Fill request client with some existing cookies per domain. First value for this
                                       option is set-cookie header and second value is url which was requested to send
                                       this set-cookie header. Example: --set-cookie "foo=bar; Domain=yolo.local"
                                       https://yolo.local. This option can be used multiple times
      --user-agent <USER_AGENT>        Update and set user agent header for requests [default: "Mozilla/5.0 (Windows NT
                                       10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0
                                       Safari/537.36"]

Decrypt Options:
      --keys <KID:KEY;...>  Keys for decrypting encrypted streams. KID:KEY should be specified in hex format
      --no-decrypt          Download encrypted streams without decrypting them. Note that --output flag is ignored if
                            this flag is used

Download Options:
      --retries <RETRIES>  Maximum number of retries to download an individual segment [default: 15]
      --no-merge           Download streams without merging them. Note that --output flag is ignored if this flag is
                           used
  -t, --threads <THREADS>  Maximum number of threads for parllel downloading of segments. Number of threads should be in
                           range 1-16 (inclusive) [default: 5]
```

## Running on Android

1. Install the [Termux](https://termux.com) app on your device, then enable storage permissions manually from its settings page. After that, run the following commands in the terminal.

```bash
$ pkg update
$ pkg upgrade
$ pkg install ffmpeg
$ ln -s /storage/emulated/0/Download Download
```

2. Install [vsd on termux](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md#android-on-termux). Currently, only *arm64-v8a* binaries pre-builts are available wjich can be installed using the following command.

```bash
curl -L https://github.com/clitic/vsd/releases/download/0.4.0/vsd-0.4.0-aarch64-linux-android.tar.xz | tar xJC $PREFIX/bin
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
3. [dash-mpd-cli](https://github.com/emarsden/dash-mpd-cli) iis a highly effective tool for downloading DASH playlists. In fact, much of vsd’s internal logic for parsing and downloading DASH content is based on this tool.
4. [ffmpeg](https://ffmpeg.org) supports direct encoding of playlists.
5. Both [streamlink](https://github.com/streamlink/streamlink) and [vlc](https://www.videolan.org/vlc) allow direct streaming of playlists.
## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
