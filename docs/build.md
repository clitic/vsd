---
icon: lucide/hammer
---

# Build Instructions

## Dependencies

- [C/C++](https://github.com/llvm/llvm-project/releases) and [Rust](https://www.rust-lang.org) compiler
- [cmake](https://cmake.org/download)
- [protoc](https://github.com/protocolbuffers/protobuf/releases)

## Cargo Features

These features can be enabled or disabled using cargo’s `--features` flag.

| Feature | Description |
|-------|-------------|
| `capture` (*default*) | Enables the `capture` sub-command. |
| `license` (*default*) | Enables the `license` sub-command. |
| `rustls-tls` (*default*) | Enables the `rustls` TLS backend for the [reqwest] crate. |
| `native-tls` | Enables the `native-tls` TLS backend for the [reqwest] crate. |
| `native-tls-vendored` | Enables the `native-tls-vendored` TLS backend for the [reqwest] crate. |

## Native Compilation

```bash
git clone https://github.com/clitic/vsd --depth 1
cd vsd
cargo build -p vsd --release
# optional - generate cli docs (docs/cli.md)
cargo run -p vsd --example vsd-docs --all-features
```

## Cross Compilation

### Android

1. Install [Android NDK](https://developer.android.com/ndk/downloads) on your system. You can also use [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) to build vsd.

    ```bash
    cd $HOME
    curl -L https://dl.google.com/android/repository/android-ndk-r27d-linux.zip -o android-ndk-r27d-linux.zip
    unzip android-ndk-r27d-linux.zip
    rm android-ndk-r27d-linux.zip
    ```

2. Add desired build targets using rustup.

    ```bash
    # arm64-v8a
    rustup target add aarch64-linux-android
    # armeabi-v7a
    rustup target add armv7-linux-androideabi
    # x86
    rustup target add i686-linux-android
    # x86_64
    rustup target add x86_64-linux-android
    ```

3. Now build with desired target.

    ```bash
    RUSTFLAGS="-C linker=aarch64-linux-android25-clang -C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib" \
      PATH="$HOME/android-ndk-r27d/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH" \
      AR="llvm-ar" \
      CC="aarch64-linux-android25-clang" \
      CXX="aarch64-linux-android25-clang++" \
      cargo build -p vsd --release --target aarch64-linux-android --no-default-features --features "license,native-tls-vendored"

    # optional - inspect binary
    llvm-readobj target/aarch64-linux-android/release/vsd --needed-libs
    ```

    !!! note

        If you are not building for termux, then you can remove `rpath` link arg.

#### Termux

You can also compile vsd directly on android using [Termux](https://f-droid.org/en/packages/com.termux).

```bash
pkg update
pkg upgrade
pkg install cmake git protobuf rust

git clone https://github.com/clitic/vsd --depth 1
cd vsd

AR=llvm-ar \
  OPENSSL_INCLUDE_DIR=$PREFIX/include/openssl \
  OPENSSL_LIB_DIR=$PREFIX/lib \
  cargo build -p vsd --release --no-default-features --features "license,native-tls"
```

### Darwin

1. Download desired [MacOSX SDK](https://github.com/joseluisq/macosx-sdks/releases).

    ```bash
    curl -L https://github.com/joseluisq/macosx-sdks/releases/download/26.1/MacOSX26.1.sdk.tar.xz | tar xJC $HOME
    ```

2. Add desired build targets using rustup.

    ```bash
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    ```

3. Now build with desired target.

    ```bash
    RUSTFLAGS="-C linker=clang -C link-arg=-target=aarch64-apple-darwin -C link-arg=-isysroot -C link-arg=$HOME/MacOSX26.1.sdk -C link-arg=-fuse-ld=lld" \
      AR="llvm-ar" \
      CC="clang -target=aarch64-apple-darwin -isysroot $HOME/MacOSX26.1.sdk -fuse-ld=lld" \
      CXX="clang++ -target=aarch64-apple-darwin -isysroot $HOME/MacOSX26.1.sdk -fuse-ld=lld" \
      cargo build -p vsd --release --target aarch64-apple-darwin
    
    # optional - inspect binary
    llvm-readobj target/aarch64-apple-darwin/release/vsd --macho-version-min --needed-libs
    ```

### Linux (MUSL)

1. Install [Zig](https://ziglang.org/download) and [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) on ypur system.

    ```bash
    cargo install cargo-zigbuild
    ```

2. Add desired build targets using rustup.

    ```bash
    rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl
    ```

3. Now build with desired target using cargo-zigbuild.

    ```bash
    cargo zigbuild -p vsd --release --target x86_64-unknown-linux-musl

    # optional - inspect binary
    llvm-readobj target/x86_64-unknown-linux-musl/release/vsd --needed-libs
    ```

### Windows (MSVC)

1. Install [cargo-xwin](https://github.com/rust-cross/cargo-xwin) on your system. 

    ```bash
    cargo install cargo-xwin
    ```

2. Add desired build targets using rustup.

    ```bash
    rustup target add aarch64-pc-windows-msvc x86_64-pc-windows-msvc
    ```

3. Now build with desired target using cargo-xwin.

    ```bash
    cargo xwin build -p vsd --release --target x86_64-pc-windows-msvc

    # optional - inspect binary
    llvm-readobj target/x86_64-pc-windows-msvc/release/vsd.exe --needed-libs
    ```

[reqwest]: https://docs.rs/reqwest/latest/reqwest/#optional-features
