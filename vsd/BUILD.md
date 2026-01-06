# Build Instructions

## Dependencies

- C/C++ Compiler
- [cmake](https://cmake.org/download)
- [protoc](https://github.com/protocolbuffers/protobuf)
- [reqwest](https://github.com/seanmonstar/reqwest#requirements) (crate)
- [rust](https://www.rust-lang.org)
- [rustfmt](https://github.com/rust-lang/rustfmt)

## Cargo Features

These features can be turned on or off by using cargo's `--features` flag.

1. `browser` (*default*): Enable `capture` subcommand.
2. `rustls` (*default*): Enable `rustls` feature of [reqwest] crate.
3. `native-tls`: Enable `native-tls` feature of [reqwest] crate.

## Any Target

```bash
git clone https://github.com/clitic/vsd --recursive --depth 1
cd vsd
cargo build -p vsd --release
```

## Android (On Linux)

1. Install [Android NDK](https://developer.android.com/ndk/downloads).

```bash
$ wget https://dl.google.com/android/repository/android-ndk-r27c-linux.zip
$ unzip android-ndk-r27c-linux.zip
$ rm android-ndk-r27c-linux.zip
```

2. Add rustup *aarch64-linux-android* target.

```bash
$ rustup target add aarch64-linux-android
```

3. Now build with *aarch64-linux-android target*. `rpath` link arg can be removed if you do not want to support termux. You can also use [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) to build vsd.

```bash
$ PATH=$HOME/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
    AR=llvm-ar \
    CC=aarch64-linux-android25-clang \
    CXX=aarch64-linux-android25-clang++ \
    RUSTFLAGS="-C linker=aarch64-linux-android25-clang -C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib" \
    cargo build -p vsd --release --target aarch64-linux-android --no-default-features --features "rustls"
```

4. Inspect for linked libraries.

```bash
$ llvm-readelf target/aarch64-linux-android/release/vsd --needed-libs
```

## Android (On Termux)

```bash
$ pkg update
$ pkg upgrade
$ pkg install git protobuf rust
$ git clone https://github.com/clitic/vsd --recursive --depth 1
$ cd vsd
$ AR=llvm-ar \
    OPENSSL_INCLUDE_DIR=$PREFIX/include/openssl
    OPENSSL_LIB_DIR=$PREFIX/lib \
    OPENSSL_NO_VENDOR=true \
    cargo build -p vsd --release --no-default-features --features "native-tls"
```

## Darwin (On Linux via osxcross)

1. Install [osxcross](https://github.com/tpoechtrager/osxcross) toolchain.

```bash
$ git clone https://github.com/tpoechtrager/osxcross
$ curl -L https://github.com/joseluisq/macosx-sdks/releases/download/15.4/MacOSX15.4.sdk.tar.xz -o osxcross/tarballs/MacOSX15.4.sdk.tar.xz
$ cd osxcross
$ sudo apt install bzip2 clang cmake cpio git libssl-dev libxml2-dev llvm-dev lzma-dev patch python3 uuid-dev zlib1g-dev xz-utils
$ UNATTENDED=1 ./build.sh
```

2. Add rustup *aarch64-apple-darwin* target.

```bash
$ rustup target add aarch64-apple-darwin
```

3. Now build with *aarch64-apple-darwin* target.

```bash
$ PATH=$HOME/osxcross/target/bin:$PATH \
    AR=aarch64-apple-darwin24.4-ar \
    CC=aarch64-apple-darwin24.4-clang \
    CXX=aarch64-apple-darwin24.4-clang++ \
    RUSTFLAGS="-C linker=aarch64-apple-darwin24.4-clang" \
    CRATE_CC_NO_DEFAULTS=true \
    cargo build -p vsd --release --target aarch64-apple-darwin
```

4. Inspect for linked libraries.

```bash
$ llvm-readobj target/aarch64-apple-darwin/release/vsd --macho-version-min --needed-libs
```

## Linux with MUSL (On Linux)

1. Build and install musl cross toolchain using [musl-cross-make](https://github.com/richfelker/musl-cross-make).

```bash
$ git clone https://github.com/richfelker/musl-cross-make --depth 1
$ cd musl-cross-make
$ TARGET=x86_64-linux-musl make install
```

2. Find and delete *libstdc++.so* for static linking else keep it.

```bash
$ find musl-cross-make/output/**/*/libstdc++.so*
$ rm musl-cross-make/output/**/*/libstdc++.so*
```

3. Add rustup *x86_64-unknown-linux-musl* target.

```bash
$ rustup target add x86_64-unknown-linux-musl
```

4. Now build with *x86_64-unknown-linux-musl* target.

```bash
$ PATH=$HOME/musl-cross-make/output/bin:$PATH \
    AR=x86_64-linux-musl-ar \
    CC=x86_64-linux-musl-gcc \
    CXX=x86_64-linux-musl-g++ \
    RUSTFLAGS="-C linker=x86_64-linux-musl-gcc" \
    cargo build -p vsd --release --target x86_64-unknown-linux-musl
```

5. Inspect for linked libraries.

```bash
$ PATH=$HOME/musl-cross-make/output/bin:$PATH x86_64-linux-musl-readelf target/x86_64-unknown-linux-musl/release/vsd --dynamic
```

## Linux with MUSL (On Linux via cargo-zigbuild)

1. Install [zig](https://ziglang.org/download) and [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild).

```bash
$ cargo install cargo-zigbuild
```

2. Add rustup *x86_64-unknown-linux-musl* target.

```bash
$ rustup target add x86_64-unknown-linux-musl
```

3. Now build with *x86_64-unknown-linux-musl* target using cargo-zigbuild.

```bash
$ cargo zigbuild -p vsd --release --target x86_64-unknown-linux-musl
```

5. Inspect for linked libraries.

```bash
$ llvm-readelf target/x86_64-unknown-linux-musl/release/vsd --needed-libs
```

## Windows with MSVC (On Linux via cargo-xwin)

1. Install [cargo-xwin](https://github.com/rust-cross/cargo-xwin).

```bash
$ cargo install cargo-xwin
```

2. Add rustup *x86_64-pc-windows-msvc* target.

```bash
$ rustup target add x86_64-pc-windows-msvc
```

3. Now build with *x86_64-pc-windows-msvc* target using cargo-xwin.

```bash
$ cargo xwin build -p vsd --release --target x86_64-pc-windows-msvc
```

5. Inspect for linked libraries.

```bash
$ llvm-readelf target/x86_64-pc-windows-msvc/release/vsd.exe --needed-libs
```

[reqwest]: https://docs.rs/reqwest/latest/reqwest/#optional-features
