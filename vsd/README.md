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

<p align="center">
  <a href="#Installation">Installation</a>
  &nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;
  <a href="#Usage">Usage</a>
  &nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;
  <a href="https://colab.research.google.com/github/clitic/vsd/blob/main/vsd-on-colab.ipynb">Try Without Install</a>
</p>

**v**ideo **s**tream **d**ownloader is a command line program to download video streams served over HTTP from websites, [HLS](https://howvideo.works/#hls) and [DASH](https://howvideo.works/#dash) playlists.

<p align="center">
  <img src="https://github.com/clitic/vsd/blob/main/vsd/images/showcase.gif">
</p>

## Features

- [x] Capturing network requests and collecting .m3u8, .mpd and subtitles from websites and save them locally.
- [x] Muxing streams to single video container using ffmpeg.
- [x] Singular progress bar for complete download process like an normal file download with realtime file size estimations.
- [x] Supports `AES-128` and `CENC` playlists decryption.
- [x] Supports HLS and DASH
- [x] Supports downloading in multiple threads.
- [ ] GUI (maybe in future)
- [ ] Supports [SAMPLE-AES](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/HLS_Sample_Encryption/Encryption/Encryption.html) playlist decryption.
- [ ] Live stream download (wip)

<a href="#Help">See More</a>

## Installation
  
Dependencies

- [ffmpeg](https://www.ffmpeg.org/download.html) (optional, *recommended*) only required for transmuxing and transcoding streams.
- [chrome](https://www.google.com/chrome) / [chromium](https://www.chromium.org/getting-involved/download-chromium/) (optional) only required for `capture` and `collect` subcommands. 

Visit [releases](https://github.com/clitic/vsd/releases) for prebuilt binaries. Download and extract archive and then copy vsd binary to any path. Now add that path to your `PATH` environment variable.

| Host                | Architecture | Download                                                                                                     |
|---------------------|--------------|--------------------------------------------------------------------------------------------------------------|
| Android 7+ (Termux) | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-aarch64-linux-android.tar.xz)      |
| Linux               | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-aarch64-unknown-linux-musl.tar.xz) |
| MacOS 11.7+         | aarch64      | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-aarch64-apple-darwin.tar.xz)       |
| Windows             | aarch64      | [.zip](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-aarch64-pc-windows-msvc.zip)          |
| Linux               | x86_64       | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-x86_64-unknown-linux-musl.tar.xz)  |
| MacOS 11.7+         | x86_64       | [.tar.xz](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-x86_64-apple-darwin.tar.xz)        |
| Windows             | x86_64       | [.zip](https://github.com/clitic/vsd/releases/download/0.3.1/vsd-0.3.1-x86_64-pc-windows-msvc.zip)           |

You can also install vsd through cargo by using this command. 

```bash
cargo install vsd
```

Build instructions can be found [here](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md) and changelog [here](https://github.com/clitic/vsd/blob/main/vsd/CHANGELOG.md).

Additionally, you can also install third party [gui](https://github.com/theRealCataclysm/VSD-GUI) frontend created by [theRealCataclysm](https://github.com/theRealCataclysm).

## Usage

- Downloading and saving HLS and DASH playlists to disk.

```bash
$ vsd save <url> -o video.mp4
```

> For testing purposes you can use streams from [https://test-streams.mux.dev](https://test-streams.mux.dev).

- Collecting .m3u8 (HLS), .mpd (Dash) and subtitles from a website and saving them locally.

```bash
$ vsd capture <url> --save
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
      --base-url <BASE_URL>    Base url to be used for building absolute url to segment. This flag is usually needed for local input files. By default redirected
                               playlist url is used
  -d, --directory <DIRECTORY>  Change directory path for temporarily downloaded files. By default current working directory is used
  -o, --output <OUTPUT>        Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg. Note that existing files will be overwritten and
                               downloaded streams will be deleted
      --parse                  Parse playlist and returns it in json format. Note that `--output` flag is ignored when this flag is used
      --color <COLOR>          When to output colored text [default: auto] [possible values: auto, always, never]
      --raw-prompts            Raw style input prompts for old and unsupported terminals
  -h, --help                   Print help

Automation Options:
      --prefer-audio-lang <PREFER_AUDIO_LANG>  Preferred language when multiple audio streams with different languages are available. Must be in RFC 5646 format (eg.
                                               fr or en-AU). If a preference is not specified and multiple audio streams are present, the first one listed in the
                                               manifest will be downloaded
      --prefer-subs-lang <PREFER_SUBS_LANG>    Preferred language when multiple subtitles streams with different languages are available. Must be in RFC 5646 format
                                               (eg. fr or en-AU). If a preference is not specified and multiple subtitles streams are present, the first one listed in
                                               the manifest will be downloaded
  -q, --quality <WIDTHxHEIGHT|HEIGHTp>         Automatic selection of some standard resolution streams with highest bandwidth stream variant from playlist. If
                                               matching resolution of WIDTHxHEIGHT is not found then only resolution HEIGHT would be considered for selection. comman
                                               values: [lowest, min, 144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k, 1440p, qhd, 4k, 8k, highest, max] [default:
                                               highest]
      --skip-prompts                           Skip user input prompts and proceed with defaults

Client Options:
      --cookies <COOKIES>              Fill request client with some existing cookies value. Cookies value can be same as document.cookie or in json format same as
                                       puppeteer
      --header <KEY> <VALUE>           Custom headers for requests. This option can be used multiple times
      --no-certificate-checks          Skip checking and validation of site certificates
      --proxy <PROXY>                  Set http(s) / socks proxy address for requests
      --set-cookie <SET_COOKIE> <URL>  Fill request client with some existing cookies per domain. First value for this option is set-cookie header and second value is
                                       url which was requested to send this set-cookie header. Example `--set-cookie "foo=bar; Domain=yolo.local" https://yolo.local`.
                                       This option can be used multiple times
      --user-agent <USER_AGENT>        Update and set user agent header for requests [default: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML,
                                       like Gecko) Chrome/112.0.0.0 Safari/537.36"]

Decrypt Options:
      --all-keys           Use all supplied keys for decryption instead of using keys which matches with default kid only
  -k, --key <KEY|KID:KEY>  Keys for decrypting encrypted streams. If streams are encrypted with a single key then there is no need to specify key id else specify
                           decryption key in format KID:KEY. KEY value can be specified in hex, base64 or file format. This option can be used multiple times
      --no-decrypt         Download encrypted streams without decrypting them. Note that --output flag is ignored if this flag is used

Download Options:
      --retry-count <RETRY_COUNT>  Maximum number of retries to download an individual segment [default: 15]
      --no-merge                   Download streams without merging them. Note that --output flag is ignored if this flag is used
  -t, --threads <THREADS>          Maximum number of threads for parllel downloading of segments. Number of threads should be in range 1-16 (inclusive) [default: 5]
```

## Alternatives

List of alternatives to vsd:

1. [N_m3u8DL-RE](https://github.com/nilaoda/N_m3u8DL-RE) is the best alternative to vsd. It also supports live playlist which vsd doesn't. It doesn't come with features like *capture*.
2. [N_m3u8DL-CLI](https://github.com/nilaoda/N_m3u8DL-CLI) is also good but it is not cross platform.
3. [m3u8-downloader](https://github.com/llychao/m3u8-downloader) is also good but it has very few customizable options.
4. [webvideo-downloader](https://github.com/jaysonlong/webvideo-downloader) opens up the website using chrome and then captures m3u8 requests. vsd's *capture* command is closest to this functionality.
5. [dash-mpd-cli](https://github.com/emarsden/dash-mpd-cli) is very good for downloading DASH playlists. Also, most of the vsd internals for parsing and downloading DASH playlists is taken for it's main project.

## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
