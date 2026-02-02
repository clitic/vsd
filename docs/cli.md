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
| `capture` | Capture playlist requests from a website |
| `extract` | Extract subtitles from a fragmented MP4 file |
| `license` | Request content keys from a license server |
| `merge` | Merge multiple media segments into a single file |
| `save` | Download streams from DASH or HLS playlist |

**Global Options:**

| Flag | Description |
|------|-------------|
| `--color` | When to use colored output<br>*Possible values:* `auto`, `always`, `never`<br>*Default:* `auto` |
| `-q, --quiet` | Suppress all output except errors |
| `-v, --verbose` | Increase verbosity: `-v` (debug), `-vv` (trace).<br><br>The default log level is `info`. |

[↑ Back to top](#command-overview)

### `vsd capture`

Capture playlist requests from a website.

Requires any one of these browsers:

- [chrome](https://www.google.com/chrome)
- [chromium](https://www.chromium.org/getting-involved/download-chromium)

This command launches an automated browser instance and listen on network requests. Behavior may vary, and it may not work as expected on all websites. This is equivalent to manually doing:

Inspect -> Network -> Fetch/XHR -> Filter by extension -> Copy as cURL (bash)

```
vsd capture [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: HTTP(S):// *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `--cookies` | Launch browser with cookies (netscape cookie file) |
| `--extensions` | List of file extensions to be filtered out separated by comma<br>*Default:* `.m3u,.m3u8,.mpd,.vtt,.ttml,.srt` |
| `--headless` | Launch browser in headless mode (without a window) |
| `--proxy` | Launch browser with a proxy |
| `--resource-types` | List of resource types to be filtered out separated by comma<br>*Possible values:* `document`, `stylesheet`, `image`, `media`, `font`, `script`, `texttrack`, `xhr`, `fetch`, `prefetch`, `eventsource`, `websocket`, `manifest`, `signedexchange`, `ping`, `cspviolationreport`, `preflight`, `fedcm`, `other`<br>*Default:* `fetch,xhr` |
| `--save-cookies` | Save browser cookies in cookies.txt (netscape cookie file) |

[↑ Back to top](#command-overview)

### `vsd extract`

Extract subtitles from a fragmented MP4 file

```
vsd extract [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: Path to an MP4 file containing WVTT (WebVTT) or STPP (TTML) subtitle boxes. For fragmented MP4 files split across multiple segments, use the `merge` sub-command first to combine them into a single file *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `-c, --codec` | Output subtitle format<br>*Possible values:* `subrip`, `webvtt`<br>*Default:* `webvtt` |
| `-o, --output` | Destination file path for extracted subtitles.<br><br>If `provided`, the codec is inferred from the file extension (`.srt` or `.vtt`). If `omitted`, subtitles are printed to stdout. |

[↑ Back to top](#command-overview)

### `vsd license`

Request content keys from a license server

```
vsd license [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: INIT_PATH | PLAYLIST_URL | BASE64_PSSH *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `-H, --header` | Additional headers for license request in same format as curl.<br><br>This option can be used multiple times. |

**Playready Options:**

| Flag | Description |
|------|-------------|
| `--playready-device` | Path to the playready device (.prd) file |
| `--playready-url` | Playready license server URL |
| `--skip-playready` | Skip playready license request |

**Widevine Options:**

| Flag | Description |
|------|-------------|
| `--widevine-device` | Path to the widevine device (.wvd) file |
| `--widevine-url` | Widevine license server URL |
| `--skip-widevine` | Skip widevine license request |

[↑ Back to top](#command-overview)

### `vsd merge`

Merge multiple media segments into a single file

```
vsd merge [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: Glob patterns for input files (e.g., `*.ts`, `segment_*.m4s`) *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `-o, --output` | Destination path for the merged output file |
| `-t, --type` | Merge strategy to use.<br><br>`binary` performs a raw byte concatenation, while `ffmpeg` uses ffmpeg's concat demuxer for container-aware merging.<br>*Possible values:* `binary`, `ffmpeg`<br>*Default:* `binary` |

[↑ Back to top](#command-overview)

### `vsd save`

Download streams from DASH or HLS playlist

```
vsd save [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: HTTP(S):// | .M3U8 | .MPD *(required)*

**Options:**

| Flag | Description |
|------|-------------|
| `--base-url` | Base URL for resolving relative segment paths.<br><br>Required for local playlist files. For remote playlists, the final redirected URL is used by default. |
| `-d, --directory` | Working directory for temporary segment files.<br><br>Defaults to the current directory. |
| `-o, --output` | Mux downloaded streams into a video container using ffmpeg (`.mp4`, `.mkv`, etc.).<br><br>Overwrites existing files and deletes intermediate stream files after muxing. |
| `--parse` | Output parsed playlist metadata as JSON instead of downloading |
| `--subs-codec` | Subtitle codec to use when muxing with ffmpeg.<br><br>Defaults to `mov_text` for `.mp4` containers, `copy` for others. |

**Automation Options:**

| Flag | Description |
|------|-------------|
| `-i, --interactive` | Enable interactive stream selection with styled prompts |
| `-I, --interactive-raw` | Enable interactive stream selection with plain text prompts |
| `-l, --list-streams` | Display all available streams without downloading |
| `-s, --select-streams` | Stream selection filters for automatic mode.<br><br>SYNTAX:<br><br>`v={}:a={}:s={}` where `{}` (in priority order) can contain<br><br>\|> all: select all streams.<br>\|> skip: skip all streams or select inverter.<br>\|> 1,2: indices obtained by --list-streams flag.<br>\|> 1080p,1280x720: stream resolution.<br>\|> en,fr: stream language.<br><br>EXAMPLES:<br><br>\|> 1,2,3 (indices 1, 2, and 3)<br>\|> v=skip:a=skip:s=all (all sub streams)<br>\|> a:en:s=en (prefer en lang)<br>\|> v=1080p:a=all:s=skip (1080p with all aud streams)<br><br>*Default:* `v=best:s=en` |

**Client Options:**

| Flag | Description |
|------|-------------|
| `--cookies` | Path to a netscape cookie file for authenticated requests |
| `-H, --header` | Additional headers for requests in same format as curl.<br><br>This option can be used multiple times. |
| `--no-certificate-checks` | Disable TLS certificate verification (insecure) |
| `--proxy` | Proxy server URL (HTTP, HTTPS, or SOCKS) |
| `--query` | Additional query parameters for requests |

**Decrypt Options:**

| Flag | Description |
|------|-------------|
| `--keys` | Decryption keys in `KID:KEY;…` hex format |
| `--no-decrypt` | Skip decryption and download encrypted streams as-is.<br><br>Ignores `--output` when enabled. |

**Download Options:**

| Flag | Description |
|------|-------------|
| `--no-merge` | Skip segment merging and keep individual files.<br><br>Ignores `--output` when enabled. |
| `--retries` | Maximum retry attempts per segment<br>*Default:* `10` |
| `-t, --threads` | Number of concurrent download threads (1–16)<br>*Default:* `5` |

[↑ Back to top](#command-overview)

