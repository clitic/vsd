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

Visit [releases](https://github.com/clitic/vsd/releases) for prebuilt binaries. Download and extract archive and copy vsd binary to any path. Now add that path to your `PATH` environment variable. Build instructions can be found [here](https://github.com/clitic/vsd/blob/main/vsd/BUILD.md) and changelog [here](https://github.com/clitic/vsd/blob/main/vsd/CHANGELOG.md).

| Host                | Architecture | Download                                                                                                      | Install                                                                                |
|---------------------|--------------|---------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------|
| Android 7+ (Termux) | aarch64      | [.tar.gz](https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-aarch64-linux-android.tar.gz)     | [command](https://github.com/clitic/vsd/blob/main/vsd/INSTALL.md#android-7-termux-aarch64) |
| Linux               | x86_64       | [.tar.gz](https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-x86_64-unknown-linux-musl.tar.gz) | [command](https://github.com/clitic/vsd/blob/main/vsd/INSTALL.md#linux-x86_64)             |
| MacOS 12.3+         | x86_64       | [.tar.gz](https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-x86_64-apple-darwin.tar.gz)       | [command](https://github.com/clitic/vsd/blob/main/vsd/INSTALL.md#macos-123-x86_64)         |
| Windows             | x86_64       | [.zip](https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-x86_64-pc-windows-msvc.zip)          |                                                                                        |

## Usage

For quick testing purposes you may use [https://test-streams.mux.dev](https://test-streams.mux.dev) as direct input. These streams are used by [hls.js](https://github.com/video-dev/hls.js) for testing purposes.

- Downloading and saving HLS and DASH playlists to disk.

```bash
$ vsd save <url> -o video.mp4
```

- Collecting .m3u8 (HLS), .mpd (Dash) and subtitles from a website and saving them locally.

```bash
$ vsd collect <url>
```

## Help

```bash
$ vsd --help
```

```
Download video streams served over HTTP from websites, HLS and DASH playlists

Usage: vsd.exe <COMMAND>

Commands:
  capture  Capture requests made to fetch playlists
  collect  Collect playlists and subtitles from a website and save them locally
  decrypt  Decrypt encrypted streams using keys
  extract  Extract subtitles embedded inside an mp4 file
  merge    Merge multiple segments to a single file
  save     Download and save HLS and DASH playlists to disk
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information (use `--help` for more detail)
  -V, --version  Print version information
```

```bash
$ vsd save --help
```

```
Download and save HLS and DASH playlists to disk

Usage: vsd.exe save [OPTIONS] <INPUT>

Arguments:
  <INPUT>  http(s):// | .m3u8 | .m3u | .mpd | .xml

Options:
  -a, --alternative
          Download alternative audio or subtitles stream from playlist instead all streams. For
          downloading video stream only, use `--skip` flag
      --baseurl <BASEURL>
          Base url for all segments. Usually needed for local m3u8 file
  -d, --directory <DIRECTORY>
          Change directory path for temporarily downloaded files. By default current working
          directory is used
  -k, --key <<KID:(base64:)KEY>|(base64:)KEY>
          Decryption keys for decrypting CENC encrypted streams. Key value should be specified in
          hex. Use `base64:` prefix if key is in base64 format. Streams encrypted with a single key
          can use `--key base64:MhbcGzyxPfkOsp3FS8qPyA==` like key format. Streams encrypted with
          multiple keys can use `--key
          eb676abbcb345e96bbcf616630f1a3da:100b6c20940f779a4589152b57d2dacb like key format. This
          option can be used multiple times
  -o, --output <OUTPUT>
          Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg. Note that
          existing files will be overwritten and downloaded streams will be deleted
      --prefer-audio-lang <PREFER_AUDIO_LANG>
          Preferred language when multiple audio streams with different languages are available.
          Must be in RFC 5646 format (eg. fr or en-AU). If a preference is not specified and
          multiple audio streams are present, the first one listed in the manifest will be
          downloaded
      --prefer-subs-lang <PREFER_SUBS_LANG>
          Preferred language when multiple subtitles streams with different languages are available.
          Must be in RFC 5646 format (eg. fr or en-AU). If a preference is not specified and
          multiple subtitles streams are present, the first one listed in the manifest will be
          downloaded
  -q, --quality <WIDTHxHEIGHT>
          Automatic selection of some standard resolution streams with highest bandwidth stream
          variant from playlist. possible values: [144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k,
          1440p, qhd, 4k, 8k, highest, max, select-later] [default: select-later]
      --raw-prompts
          Raw style input prompts for old and unsupported terminals
      --retry-count <RETRY_COUNT>
          Maximum number of retries to download an individual segment [default: 15]
  -s, --skip
          Skip downloading and muxing alternative streams
  -t, --threads <THREADS>
          Maximum number of threads for parllel downloading of segments. Number of threads should be
          in range 1-16 (inclusive) [default: 5]
  -h, --help
          Print help information

Client Options:
      --cookies <COOKIES> <URL>
          Enable cookie store and fill it with some existing cookies. Example `--cookies "foo=bar;
          Domain=yolo.local" https://yolo.local`. This option can be used multiple times
      --enable-cookies
          Enable cookie store which allows cookies to be stored
      --header <KEY> <VALUE>
          Custom headers for requests. This option can be used multiple times
      --proxy-address <PROXY_ADDRESS>
          Set http or https proxy address for requests
      --user-agent <USER_AGENT>
          Update and set custom user agent for requests [default: "Mozilla/5.0 (Windows NT 10.0;
          Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36"]
```

## Alternatives

List of alternatives to vsd:

1. [N_m3u8DL-RE](https://github.com/nilaoda/N_m3u8DL-RE) is the best alternative to vsd. It also supports live playlist which vsd doesn't. It doesn't come with features like *capture* and *collect*. Also, CENC encrypted playlist decryption is slow.
2. [N_m3u8DL-CLI](https://github.com/nilaoda/N_m3u8DL-CLI) is good but it is not cross platform.
3. [m3u8-downloader](https://github.com/llychao/m3u8-downloader) is also good but it has very few customizable options.
4. [webvideo-downloader](https://github.com/jaysonlong/webvideo-downloader) opens websites using chrome and captures the m3u8 links and then downloads it. A similar functionality can achieved with vsd too by using *capture* and *collect* subcommands.
5. [dash-mpd-cli](https://github.com/emarsden/dash-mpd-cli) is very good for downloading DASH playlists. Also most of the vsd functionalities for parsing and downloading DASH playlists is taken for it's main project.

## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
