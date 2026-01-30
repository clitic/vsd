---
icon: lucide/rocket
---
  
## Dependencies

- [ffmpeg](https://www.ffmpeg.org/download.html) (optional, *recommended*) required for transmuxing and transcoding streams.
- [chrome](https://www.google.com/chrome) / [chromium](https://www.chromium.org/getting-involved/download-chromium/) (optional) needed only for the capture sub-command. 

## Pre-built Binaries

Visit the [releases page](https://github.com/clitic/vsd/releases) for pre-built binaries or grab the [latest CI builds](https://nightly.link/clitic/vsd/workflows/build/main).
Download and extract the archive, then copy the vsd binary to a directory of your choice.
Finally, add that directory to your system's `PATH` environment variable.

=== ":fontawesome-brands-windows: Windows"

    Downloads and extracts the binary to your current directory.

    === "x86_64"

        ```powershell
        irm https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-x86_64-pc-windows-msvc.zip -OutFile vsd.zip; Expand-Archive vsd.zip -DestinationPath . -Force; rm vsd.zip
        ```

    === "arm64"
    
        ```powershell
        irm https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-pc-windows-msvc.zip -OutFile vsd.zip; Expand-Archive vsd.zip -DestinationPath . -Force; rm vsd.zip
        ```

=== ":fontawesome-brands-linux: Linux"

    Downloads and extracts the binary to your current directory.

    === "x86_64"

        ```bash
        curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-x86_64-unknown-linux-musl.tar.xz | tar xJC .
        ```

    === "arm64"
    
        ```bash
        curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-unknown-linux-musl.tar.xz | tar xJC .
        ```

=== ":fontawesome-brands-apple: MacOS"

    Downloads and extracts the binary to your current directory.

    === "via Homebrew"

        ```bash
        brew install vsd
        ```

    === "x86_64"

        ```bash
        curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-x86_64-apple-darwin.tar.xz | tar xJC .
        ```

    === "arm64"
    
        ```bash
        curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-apple-darwin.tar.xz | tar xJC .
        ```

=== ":fontawesome-brands-android: Android"

    Requires [Termux](https://f-droid.org/en/packages/com.termux). Downloads and extracts the binary to $PREFIX/bin.

    === "arm64"

        ```bash
        curl -L https://github.com/clitic/vsd/releases/download/vsd-0.4.3/vsd-0.4.3-aarch64-linux-android.tar.xz | tar xJC $PREFIX/bin
        ```

## Install via Cargo

You can also install vsd using cargo.

```bash
$ cargo install vsd
```
