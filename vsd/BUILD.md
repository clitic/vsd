# Building From Source

## Build Dependencies

- C/C++ Compiler
- [openssl](https://docs.rs/openssl/latest/openssl) (crate)
- [protoc](https://github.com/protocolbuffers/protobuf)
- [rust](https://www.rust-lang.org)

## Cargo Features

These features can be turned on or off by using cargo's `--features` flag.

1. `browser` (*default*): Enable `capture` subcommand.
2. `native-tls` (*default*): Enable `native-tls` feature of [reqwest] crate.
3. `rustls-tls-native-roots`: Enable `rustls-tls-native-roots` feature of [reqwest] crate.
4. `rustls-tls-webpki-roots`: Enable `rustls-tls-webpki-roots` feature of [reqwest] crate.

## Any Target

```bash
git clone https://github.com/clitic/vsd --recursive --depth 1
cd vsd
cargo build -p vsd --release
```

## Android (On Linux)

1. Install Android [NDK](https://developer.android.com/ndk/downloads).

```bash
$ wget https://dl.google.com/android/repository/android-ndk-r22b-linux-x86_64.zip
$ unzip android-ndk-r22b-linux-x86_64.zip
$ rm android-ndk-r22b-linux-x86_64.zip
```

2. Add rustup target aarch64-linux-android.

```bash
$ rustup target add aarch64-linux-android
$ printf '\n[target.aarch64-linux-android]\nlinker = "aarch64-linux-android30-clang"\n' >> $HOME/.cargo/config.toml
```

3. Now build with aarch64-linux-android target. `RUSTFLAGS` variable can be removed if you do not want to support termux.

```bash
$ PATH=/content/android-ndk-r22b/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
    RUSTFLAGS="-C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib -C link-args=-Wl,--enable-new-dtags" \
    cargo build -p vsd --release --target aarch64-linux-android --no-default-features --features "rustls-tls"
```

## Android (On Termux)

```bash
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

## Darwin (On Linux)

1. Install [osxcross](https://github.com/tpoechtrager/osxcross) toolchain.

```bash
$ git clone https://github.com/tpoechtrager/osxcross
$ curl -L https://github.com/joseluisq/macosx-sdks/releases/download/13.1/MacOSX13.1.sdk.tar.xz -o osxcross/tarballs/MacOSX13.1.sdk.tar.xz
$ cd osxcross
$ ./tools/get_dependencies.sh
$ apt install llvm
$ UNATTENDED=1 SDK_VERSION=13.1 ./build.sh
$ # compiler-rt (optional)
$ ENABLE_COMPILER_RT_INSTALL=1 ./build_compiler_rt.sh
```

2. Add rustup target x86_64-apple-darwin.

```bash
$ rustup target add x86_64-apple-darwin
$ printf '\n[target.x86_64-apple-darwin]\nlinker = "x86_64-apple-darwin21.4-clang"\n' >> $HOME/.cargo/config.toml
```

3. Now build with x86_64-apple-darwin target.

```bash
$ PATH=/content/osxcross/target/bin:$PATH \
    LD_LIBRARY_PATH=/content/osxcross/target/lib:$LD_LIBRARY_PATH \
    MACOSX_DEPLOYMENT_TARGET=13.1 \
    CC=x86_64-apple-darwin21.4-clang \
    CXX=x86_64-apple-darwin21.4-clang++ \
    AR=x86_64-apple-darwin21.4-ar \
    cargo build -p vsd --release --target x86_64-apple-darwin
$ llvm-readobj ./target/x86_64-apple-darwin/release/vsd --macho-version-min --needed-libs
```

## Linux with MUSL (On Linux)

1. Build musl cross toolchain using [musl-cross-make](https://github.com/richfelker/musl-cross-make).

```bash
$ git clone https://github.com/richfelker/musl-cross-make --depth 1
$ cd musl-cross-make
$ TARGET=x86_64-linux-musl make install
```

2. Find and delete `libstdc++.so` for static linking else keep it.

```bash
$ find musl-cross-make/output/**/*/libstdc++.so*
$ rm musl-cross-make/output/**/*/libstdc++.so*
```

3. Add rustup target x86_64-unknown-linux-musl.

```bash
$ rustup target add x86_64-unknown-linux-musl
$ printf '\n[target.x86_64-unknown-linux-musl]\nlinker = "x86_64-linux-musl-gcc"\n' >> $HOME/.cargo/config.toml
```

4. Now build with x86_64-unknown-linux-musl target.

```bash
$ PATH=/content/musl-cross-make/output/bin:$PATH \
    CC=x86_64-linux-musl-gcc \
    CXX=x86_64-linux-musl-g++ \
    AR=x86_64-linux-musl-ar \
    cargo build -p vsd --release --target x86_64-unknown-linux-musl --no-default-features --features "browser,rustls-tls-webpki-roots"
$ PATH=/content/musl-cross-make/output/bin:$PATH x86_64-linux-musl-readelf ./target/x86_64-unknown-linux-musl/release/vsd --dynamic
```

[reqwest]: https://docs.rs/reqwest/latest/reqwest/#optional-features
