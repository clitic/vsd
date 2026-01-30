---
icon: lucide/mouse-pointer-2
---

# Usage

Below are some example commands. For additional usage details, see [cli reference](https://clitic.github.io/vsd/cli).

- Capture playlists and subtitles from a website.

    ```bash
    vsd capture <url> --save-cookies
    ```

    !!! info
        The saved cookies can be used as `--cookies cookies.txt` with `save` sub-command later on.

- Download playlists. ([test streams](https://test-streams.mux.dev))

    ```bash
    vsd save <url> -o video.mp4
    ```

    !!! info
        Use `-i, --interactive` flag to open an interactive session.

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
