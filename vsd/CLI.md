# Command-Line Help for `vsd`

This document contains the help content for the `vsd` command-line program.

**Command Overview:**

* [`vsd`↴](#vsd)
* [`vsd capture`↴](#vsd-capture)
* [`vsd extract`↴](#vsd-extract)
* [`vsd license`↴](#vsd-license)
* [`vsd merge`↴](#vsd-merge)
* [`vsd save`↴](#vsd-save)

## `vsd`

Download video streams served over HTTP from websites, DASH (.mpd) and HLS (.m3u8) playlists.

**Usage:** `vsd [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `capture` — Capture playlists and subtitles requests from a website
* `extract` — Extract subtitles from mp4 boxes
* `license` — Request content keys from a license server
* `merge` — Merge multiple segments to a single file
* `save` — Download DASH and HLS playlists

###### **Options:**

* `--color <COLOR>` — When to output colored text

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `-q`, `--quiet` — Silence all output and only log errors
* `-v`, `--verbose` — Increase verbosity (-v [debug], -vv [trace]). Default logging level is set to info



## `vsd capture`

Capture playlists and subtitles requests from a website.

Requires one of the following browsers to be installed:
* chrome   - https://www.google.com/chrome
* chromium - https://www.chromium.org/getting-involved/download-chromium

This command launches an automated browser instance and listen on requests. Behavior may vary, and it may not work as expected on all websites. This is equivalent to manually doing:
Inspect -> Network -> Fetch/XHR -> Filter by extension -> Copy as cURL (bash)

**Usage:** `vsd capture [OPTIONS] <URL>`

###### **Arguments:**

* `<URL>` — http(s)://

###### **Options:**

* `--cookies <PATH>` — Launch browser with cookies loaded from a json file
* `--extensions <EXTENSIONS>` — List of file extensions to be filter out seperated by comma

  Default value: `.m3u,.m3u8,.mpd,.vtt,.ttml,.srt`
* `--headless` — Launch browser without a window
* `--proxy <PROXY>` — Launch browser with a proxy
* `--resource-types <RESOURCE_TYPES>` — List of resource types to be filter out seperated by commas.

   [possible values: document, stylesheet, image, media, font, script, texttrack, object, other, fetch, xhr]

  Default value: `fetch,xhr`
* `--save-cookies` — Save browser cookies in vsd-cookies.json file



## `vsd extract`

Extract subtitles from mp4 boxes

**Usage:** `vsd extract [OPTIONS] <INPUT>`

###### **Arguments:**

* `<INPUT>` — Path of mp4 file which either contains WVTT or STPP box. If there are multiple fragments of same mp4 file, then merge them using merge sub-command

###### **Options:**

* `-c`, `--codec <CODEC>` — Codec for output subtitles

  Default value: `webvtt`

  Possible values: `subrip`, `webvtt`




## `vsd license`

Request content keys from a license server

**Usage:** `vsd license [OPTIONS] <PATH|URL|BASE64>`

###### **Arguments:**

* `<PATH|URL|BASE64>` — PSSH data input. Can be an init file path, playlist url or base64 encoded PSSH box

###### **Options:**

* `-H`, `--header <KEY:VALUE>` — Extra headers for license request in same format as curl.

   This option can be used multiple times.
* `--playready-device <PRD>` — Path to the Playready device (.prd) file
* `--widevine-device <WVD>` — Path to the Widevine device (.wvd) file
* `--playready-url <URL>` — Playready license server URL
* `--widevine-url <URL>` — Widevine license server URL



## `vsd merge`

Merge multiple segments to a single file

**Usage:** `vsd merge [OPTIONS] --output <OUTPUT> <FILES>...`

###### **Arguments:**

* `<FILES>` — List of files (at least 2) to merge together e.g. *.ts, *.m4s etc. 

###### **Options:**

* `-o`, `--output <OUTPUT>` — Path for merged output file
* `-t`, `--type <TYPE>` — Type of merge to be performed

  Default value: `binary`

  Possible values: `binary`, `ffmpeg`




## `vsd save`

Download DASH and HLS playlists

**Usage:** `vsd save [OPTIONS] <INPUT>`

###### **Arguments:**

* `<INPUT>` — http(s):// | .mpd | .xml | .m3u8

###### **Options:**

* `--base-url <BASE_URL>` — Base url to be used for building absolute url to segment. This flag is usually needed for local input files. By default redirected playlist url is used
* `-d`, `--directory <DIRECTORY>` — Change directory path for temporarily downloaded files. By default current working directory is used
* `-o`, `--output <OUTPUT>` — Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg. Note that existing files will be overwritten and downloaded streams will be deleted
* `--parse` — Parse playlist and returns it in json format. Note that --output flag is ignored when this flag is used
* `--subs-codec <SUBS_CODEC>` — Force some specific subtitle codec when muxing through ffmpeg. By default `mov_text` is used for .mp4 and `copy` for others

  Default value: `copy`
* `-i`, `--interactive` — Prompt for custom streams selection with modern style input prompts. By default proceed with defaults
* `-I`, `--interactive-raw` — Prompt for custom streams selection with raw style input prompts. By default proceed with defaults
* `-l`, `--list-streams` — List all the streams present inside the playlist
* `-s`, `--select-streams <SELECT_STREAMS>` — Filters to be applied for automatic stream selection.

   SYNTAX: `v={}:a={}:s={}` where `{}` (in priority order) can contain
   |> all: select all streams.
   |> skip: skip all streams or select inverter.
   |> 1,2: indices obtained by --list-streams flag.
   |> 1080p,1280x720: stream resolution.
   |> en,fr: stream language.

   EXAMPLES:
   |> v=skip:a=skip:s=all (download all sub streams)
   |> a:en:s=en (prefer en lang)
   |> v=1080p:a=all:s=skip (1080p with all audio streams)

  Default value: `v=best:s=en`
* `--cookies <COOKIES>` — Fill request client with some existing cookies value. Cookies value can be same as document.cookie or in json format same as puppeteer

  Default value: `[]`
* `-H`, `--header <KEY:VALUE>` — Extra headers for requests in same format as curl.

   This option can be used multiple times.
* `--no-certificate-checks` — Skip checking and validation of site certificates
* `--proxy <PROXY>` — Set http(s) / socks proxy address for requests
* `--query <QUERY>` — Set query parameters for requests

  Default value: ``
* `--keys <KID:KEY;…>` — Keys for decrypting encrypted streams. KID:KEY should be specified in hex format

  Default value: ``
* `--no-decrypt` — Download encrypted streams without decrypting them. Note that --output flag is ignored if this flag is used
* `--no-merge` — Download streams without merging them. Note that --output flag is ignored if this flag is used
* `--retries <RETRIES>` — Maximum number of retries to download an individual segment

  Default value: `10`
* `-t`, `--threads <THREADS>` — Total number of threads for parllel downloading of segments. Number of threads should be in range 1-16 (inclusive)

  Default value: `5`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
