# Building From Source

1. Build and install [openssl](https://github.com/openssl/openssl) for your target platform.
All openssl static builds used by vsd are available on [google drive](https://drive.google.com/drive/folders/11DaFm8pWwQoGpgbEbL8DmHce9ozTjWqz).

2. Install [Rust](https://www.rust-lang.org)

3. Download or clone repository.

```bash
git clone https://github.com/clitic/vsd.git
```

## Windows

```powershell
$env:x86_64_PC_WINDOWS_MSVC_OPENSSL_DIR="C:\openssl-3.0.5-VC-WIN64A-static"
$env:x86_64_PC_WINDOWS_MSVC_NO_VENDOR=$true
$env:x86_64_PC_WINDOWS_MSVC_OPENSSL_STATIC=$true
cargo build --release
```

## Linux / MacOS

Install any C++ compiler and run cargo build command inside vsd directory.

```bash
cargo build --release
```

## Linux with MUSL (On Linux)

1. Build musl cross toolchain [musl-cross-make](https://github.com/richfelker/musl-cross-make) with C++ support.

```bash
$ curl -L https://github.com/richfelker/musl-cross-make/archive/refs/tags/v0.9.9.tar.gz | tar xz -C .
$ cd musl-cross-make-0.9.9
$ TARGET=x86_64-linux-musl make install
```

2. Find and delete `libstdc++.so` for static linking else keep it.

```bash
$ find musl-cross-make/**/*/libstdc++.so*
$ rm musl-cross-make/**/*/libstdc++.so*
```

2. Add rustup target x86_64-unknown-linux-musl.

```bash
$ rustup target add x86_64-unknown-linux-musl
$ printf '\n[target.x86_64-unknown-linux-musl]\nlinker = "x86_64-linux-musl-gcc"\n' >> $HOME/.cargo/config.toml
```

3. Now compile with target x86_64-unknown-linux-musl.

```bash
$ PATH=/content/musl-cross-make-0.9.9/output/bin:$PATH \
    CC=x86_64-linux-musl-gcc \
    CXX=x86_64-linux-musl-g++ \
    AR=x86_64-linux-musl-ar \
    x86_64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR=/content/openssl-v3.0.5-static-x86_64-linux-musl \
    x86_64_UNKNOWN_LINUX_MUSL_OPENSSL_NO_VENDOR=true \
    x86_64_UNKNOWN_LINUX_MUSL_OPENSSL_STATIC=true \
    cargo build --release --target x86_64-unknown-linux-musl
```

4. Check that binary is linking to any shared library or not.

```
$ PATH=/content/musl-cross-make-0.9.9/output/bin:$PATH x86_64-linux-musl-readelf target/x86_64-unknown-linux-musl/release/vsd --dynamic
```

## MacOS (On Linux)

1. Build [osxcross](https://github.com/tpoechtrager/osxcross) toolchain.

```bash
$ git clone https://github.com/tpoechtrager/osxcross
$ wget https://github.com/joseluisq/macosx-sdks/releases/download/12.3/MacOSX12.3.sdk.tar.xz -O osxcross/tarballs/MacOSX12.3.sdk.tar.xz
$ cd osxcross
$ ./tools/get_dependencies.sh
$ UNATTENDED=1 SDK_VERSION=12.3 ./build.sh
$ # compiler rt support (optional)
$ apt install llvm
$ ENABLE_COMPILER_RT_INSTALL=1 ./build_compiler_rt.sh
```

2. Install openssl using `osxcross-macports`.

```bash
$ PATH=/content/osxcross/target/bin:$PATH \
    MACOSX_DEPLOYMENT_TARGET=12.3 \
    osxcross-macports install openssl
```

3. Add rustup target x86_64-apple-darwin.

```bash
$ rustup target add x86_64-apple-darwin
$ printf '\n[target.x86_64-apple-darwin]\nlinker = "x86_64-apple-darwin21.4-clang"\n' >> $HOME/.cargo/config.toml
```

4. Now compile with target x86_64-apple-darwin.

```bash
$ PATH=/content/osxcross/target/bin:$PATH \
    MACOSX_DEPLOYMENT_TARGET=12.3 \
    OSXCROSS_PKG_CONFIG_NO_MP_INC=1 \
    OSXCROSS_MP_INC=1 \
    CC=x86_64-apple-darwin21.4-clang \
    CXX=x86_64-apple-darwin21.4-clang++ \
    AR=x86_64-apple-darwin21.4-ar \
    x86_64_APPLE_DARWIN_OPENSSL_DIR=/content/osxcross/target/macports/pkgs/opt/local/libexec/openssl3 \
    x86_64_APPLE_DARWIN_OPENSSL_NO_VENDOR=true \
    cargo build --release --target x86_64-apple-darwin
```

5. Check that binary is linking to any shared library or not.

```bash
$ PATH=/content/osxcross/target/bin:$PATH x86_64-apple-darwin21.4-readelf target/x86_64-apple-darwin/release/vsd --needed-libs
```

## Android (On Linux 64-bit)

1. Install [NDK](https://developer.android.com/ndk/downloads)

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

3. Now compile with target aarch64-linux-android. RUSTFLAGS can be removed if you do not want to build for termux.

```bash
$ PATH=/content/android-ndk-r22b/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
    AARCH64_LINUX_ANDROID_OPENSSL_DIR=/content/openssl-v3.0.5-static-aarch64-linux-android30 \
    AARCH64_LINUX_ANDROID_OPENSSL_NO_VENDOR=true \
    AARCH64_LINUX_ANDROID_OPENSSL_STATIC=true \
    RUSTFLAGS="-C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib -C link-args=-Wl,--enable-new-dtags" \
    cargo build --release --target aarch64-linux-android
```

## Android (On Termux)

```bash
$ pkg upgrade
$ pkg install git rust
$ git clone https://github.com/clitic/vsd
$ cd vsd
$ OPENSSL_INCLUDE_DIR=$PREFIX/include/openssl \
    OPENSSL_LIB_DIR=$PREFIX/lib \
    OPENSSL_NO_VENDOR=true \
    AR=llvm-ar \
    cargo build --release
```
