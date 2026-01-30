---
icon: lucide/terminal
---

# VSD CLI

This document contains cli reference for the `vsd` command-line program.

## Command Overview

- [`vsd`↴](#vsd)
- [`vsd capture`↴](#vsd-capture)
- [`vsd extract`↴](#vsd-extract)
- [`vsd license`↴](#vsd-license)
- [`vsd merge`↴](#vsd-merge)
- [`vsd save`↴](#vsd-save)

## `vsd`

Download video streams served over HTTP from websites, DASH (.mpd) and HLS (.m3u8) playlists.

```
vsd [OPTIONS] <COMMAND>
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `capture` | Capture playlists and subtitles requests from a website |
| `extract` | Extract subtitles from mp4 boxes |
| `license` | Request content keys from a license server |
| `merge` | Merge multiple segments to a single file |
| `save` | Download DASH and HLS playlists |

**Global Options:**

| Flag | Description |
|------|-------------|
| `--color` | When to output colored text<br>*Possible values:* `auto`, `always`, `never`<br>*Default:* `auto` |
| `-q, --quiet` | Silence all output and only log errors |
| `-v, --verbose` | Increase verbosity (-v [debug], -vv [trace]). Default logging level is set to info |

[↑ Back to top](#command-overview)

### `vsd capture`

Capture playlists and subtitles requests from a website.

Requires one of the following browsers to be installed:
* chrome   - https://www.google.com/chrome
* chromium - https://www.chromium.org/getting-involved/download-chromium

This command launches an automated browser instance and listen on requests. Behavior may vary, and it may not work as expected on all websites. This is equivalent to manually doing:
Inspect -> Network -> Fetch/XHR -> Filter by extension -> Copy as cURL (bash)

```
vsd capture [OPTIONS] <URL>
```

**Arguments:**

- `<URL>`: http(s):// *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `--cookies` | Launch browser with cookies loaded from a json file |
| `--extensions` | List of file extensions to be filter out seperated by comma<br>*Default:* `.m3u,.m3u8,.mpd,.vtt,.ttml,.srt` |
| `--headless` | Launch browser without a window |
| `--proxy` | Launch browser with a proxy |
| `--resource-types` | List of resource types to be filter out seperated by commas<br>*Possible values:* `document`, `stylesheet`, `image`, `media`, `font`, `script`, `texttrack`, `xhr`, `fetch`, `prefetch`, `eventsource`, `websocket`, `manifest`, `signedexchange`, `ping`, `cspviolationreport`, `preflight`, `fedcm`, `other`<br>*Default:* `fetch,xhr` |
| `--save-cookies` | Save browser cookies in vsd-cookies.json file |

[↑ Back to top](#command-overview)

### `vsd extract`

Extract subtitles from mp4 boxes

```
vsd extract [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: Path of mp4 file which either contains WVTT or STPP box. If there are multiple fragments of same mp4 file, then merge them using merge sub-command *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `-c, --codec` | Codec for output subtitles<br>*Possible values:* `subrip`, `webvtt`<br>*Default:* `webvtt` |

[↑ Back to top](#command-overview)

### `vsd license`

Request content keys from a license server

```
vsd license [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: PSSH data input. Can be an init file path, playlist url or base64 encoded PSSH box *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `-H, --header` | Extra headers for license request in same format as curl.<br><br>This option can be used multiple times. |
| `--playready-device` | Path to the Playready device (.prd) file |
| `--widevine-device` | Path to the Widevine device (.wvd) file |
| `--playready-url` | Playready license server URL |
| `--widevine-url` | Widevine license server URL |

[↑ Back to top](#command-overview)

### `vsd merge`

Merge multiple segments to a single file

```
vsd merge [OPTIONS] <FILES>
```

**Arguments:**

- `<FILES>`: List of files (at least 2) to merge together e.g. *.ts, *.m4s etc.  *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `-o, --output` | Path for merged output file |
| `-t, --type` | Type of merge to be performed<br>*Possible values:* `binary`, `ffmpeg`<br>*Default:* `binary` |

[↑ Back to top](#command-overview)

### `vsd save`

Download DASH and HLS playlists

```
vsd save [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: http(s):// | .mpd | .xml | .m3u8 *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `--base-url` | Base url to be used for building absolute url to segment. This flag is usually needed for local input files. By default redirected playlist url is used |
| `-d, --directory` | Change directory path for temporarily downloaded files. By default current working directory is used |
| `-o, --output` | Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg. Note that existing files will be overwritten and downloaded streams will be deleted |
| `--parse` | Parse playlist and returns it in json format. Note that --output flag is ignored when this flag is used |
| `--subs-codec` | Force some specific subtitle codec when muxing through ffmpeg. By default `mov_text` is used for .mp4 and `copy` for others<br>*Default:* `copy` |

**Automation Options:**

| Flag | Description |
|------|-------------|
| `-i, --interactive` | Prompt for custom streams selection with modern style input prompts. By default proceed with defaults |
| `-I, --interactive-raw` | Prompt for custom streams selection with raw style input prompts. By default proceed with defaults |
| `-l, --list-streams` | List all the streams present inside the playlist |
| `-s, --select-streams` | Filters to be applied for automatic stream selection.<br><br>SYNTAX: `v={}:a={}:s={}` where `{}` (in priority order) can contain<br>\|> all: select all streams.<br>\|> skip: skip all streams or select inverter.<br>\|> 1,2: indices obtained by --list-streams flag.<br>\|> 1080p,1280x720: stream resolution.<br>\|> en,fr: stream language.<br><br>EXAMPLES:<br>\|> v=skip:a=skip:s=all (download all sub streams)<br>\|> a:en:s=en (prefer en lang)<br>\|> v=1080p:a=all:s=skip (1080p with all audio streams)<br><br>*Default:* `v=best:s=en` |

**Client Options:**

| Flag | Description |
|------|-------------|
| `--cookies` | Fill request client with some existing cookies value. Cookies value can be same as document.cookie or in json format same as puppeteer |
| `-H, --header` | Extra headers for requests in same format as curl.<br><br>This option can be used multiple times. |
| `--no-certificate-checks` | Skip checking and validation of site certificates |
| `--proxy` | Set http(s) / socks proxy address for requests |
| `--query` | Set query parameters for requests |

**Decrypt Options:**

| Flag | Description |
|------|-------------|
| `--keys` | Keys for decrypting encrypted streams. KID:KEY should be specified in hex format |
| `--no-decrypt` | Download encrypted streams without decrypting them. Note that --output flag is ignored if this flag is used |

**Download Options:**

| Flag | Description |
|------|-------------|
| `--no-merge` | Download streams without merging them. Note that --output flag is ignored if this flag is used |
| `--retries` | Maximum number of retries to download an individual segment<br>*Default:* `10` |
| `-t, --threads` | Total number of threads for parllel downloading of segments. Number of threads should be in range 1-16 (inclusive)<br>*Default:* `5` |

[↑ Back to top](#command-overview)

