<h1 align="center">vsd</h1>

<p align="center">
  <img src="https://img.shields.io/github/license/clitic/vsd?style=flat-square">
  <img src="https://img.shields.io/github/repo-size/clitic/vsd?style=flat-square">
  <img src="https://img.shields.io/tokei/lines/github/clitic/vsd?style=flat-square">
</p>

<p align="center">
  <a href="#Installations">Installations</a>
  &nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;
  <a href="#Usage">Usage</a>
</p>

Command line program to download HLS video from a website, m3u8 url or from a local m3u8 file.

Know more about HLS from [howvideo.works](https://howvideo.works) and 
[wikipedia](https://en.wikipedia.org/wiki/M3U).

There are some alternatives to vsd but they lack in some features like [N_m3u8DL-CLI](https://github.com/nilaoda/N_m3u8DL-CLI) is not cross platform and [m3u8-downloader](https://github.com/llychao/m3u8-downloader) has very few customizable options.

<p align="center">
  <img src="https://github.com/clitic/vsd/blob/main/images/showcase.png">
</p>

## Features

- [x] Beautiful resolution and bandwidth based master playlist parsing.
- [x] Captures m3u8 network requests from a website.
- [x] Collects .m3u8, .mpd and subtitles from a website and save them locally.
- [x] Custom headers, proxies and cookies.
- [x] Downloads in multiple threads.
- [x] Inbuilt web scrapper for querying HLS and DASH links.
- [x] Multiple output formats which are supported by ffmpeg.
- [x] Muxing seperate video, audio and subtitle (webvtt) stream to single file.
- [x] Progressive binary merging of segments.
- [x] Realtime file size prediction.
- [x] Select standard resolution playlist like `HD`, `FHD` etc.
- [x] Supports `AES-128` playlist decryption.
- [x] Supports multiple retries.
- [x] Supports resume.
- [ ] GUI
- [ ] Supports [SAMPLE-AES](https://datatracker.ietf.org/doc/html/rfc8216#section-4.3.2.4) playlist decryption.
- [ ] Supports live stream download.

<a href="#Help">See More</a>

## Installations

Dependencies

- [ffmpeg](https://www.ffmpeg.org/download.html) (optional) only required for transmuxing and transcoding streams.
- [chrome](https://www.google.com/chrome) (optional) only required for `CHROME OPTIONS` related flag. 

Visit [releases](https://github.com/clitic/vsd/releases) for prebuilt binaries. You just need to copy that binary to any path specified in your `PATH` environment variable.

## Usage

For quick testing purposes you may use [https://test-streams.mux.dev](https://test-streams.mux.dev) as direct input. These streams are used by [hls.js](https://github.com/video-dev/hls.js) for testing purposes.

- Downloading HLS video from a website, m3u8 url or from a local m3u8 file.

```bash
$ vsd <url | .m3u8> -o video.mp4
```

> Use **-r/--resume** flag to resume a download session.

- Collecting .m3u8 (HLS), .mpd (Dash) and subtitles from a website and saving them locally. (requires [chrome](https://www.google.com/chrome))

```bash
$ vsd <url> --collect
```

## Help

```bash
$ vsd --help
```

```
vsd 0.1.0
clitic <clitic21@gmail.com>
Command line program to download HLS video from a website, m3u8 url or from a local m3u8 file.

USAGE:
    vsd.exe [OPTIONS] <INPUT>

ARGS:
    <INPUT>    url | .m3u8 | .m3u

OPTIONS:
    -a, --alternative                  Download alternative streams such as audio and subtitles
                                       streams from master playlist instead of variant video streams
    -b, --baseurl <BASEURL>            Base url for all segments. Usually needed for local m3u8 file
    -h, --help                         Print help information
    -o, --output <OUTPUT>              Path of final downloaded video stream. For file extension any
                                       ffmpeg supported format could be provided. If playlist
                                       contains alternative streams vsd will try to transmux and
                                       trancode into single file using ffmpeg
    -q, --quality <QUALITY>            Automatic selection of some standard resolution streams with
                                       highest bandwidth stream variant from master playlist
                                       [default: select] [possible values: select, sd, hd, fhd, uhd,
                                       uhd4k, max]
    -r, --resume                       Resume a download session. Download session can only be
                                       resumed if download session json file is present
        --raw-prompts                  Raw style input prompts for old and unsupported terminals
        --retry-count <RETRY_COUNT>    Maximum number of retries to download an individual segment
                                       [default: 15]
    -s, --skip                         Skip downloading and muxing alternative streams
    -t, --threads <THREADS>            Maximum number of threads for parllel downloading of
                                       segments. Number of threads should be in range 1-16
                                       (inclusive) [default: 5]
    -V, --version                      Print version information

CHROME OPTIONS:
        --capture     Launch Google Chrome to capture requests made to fetch .m3u8 (HLS) and .mpd
                      (Dash) files
        --collect     Launch Google Chrome and collect .m3u8 (HLS), .mpd (Dash) and subtitles from a
                      website and save them locally. Some websites have custom collector method for
                      other websites their is an comman collector method
        --headless    Launch Google Chrome without a window for interaction. This option must be
                      used with `--capture` or `--collect` flag only

CLIENT OPTIONS:
        --cookies <cookies> <url>
            Enable cookie store and fill it with some existing cookies. Example `--cookies "foo=bar;
            Domain=yolo.local" https://yolo.local`. This option can be used multiple times

        --enable-cookies
            Enable cookie store which allows cookies to be stored

        --header <key> <value>
            Custom headers for requests. This option can be used multiple times

        --proxy-address <PROXY_ADDRESS>
            Custom http or https proxy address for requests

        --user-agent <USER_AGENT>
            Update and set custom user agent for requests [default: "Mozilla/5.0 (Windows NT 10.0;
            Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36"]
```

## Building From Source

- Install [Rust](https://www.rust-lang.org)

- Install Openssl
    - [Linux](https://docs.rs/openssl/latest/openssl/#automatic)
    - [Windows](https://wiki.openssl.org/index.php/Binaries) - Also set `OPENSSL_DIR` environment variable.

- Clone Repository

```bash
git clone https://github.com/clitic/vsd.git
```

- Build Release (inside vsd directory)

```bash
cargo build --release
```

## License

&copy; 2022 clitic

This repository is licensed under the MIT license. See LICENSE for details.