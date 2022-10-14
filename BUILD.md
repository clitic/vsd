# Building From Source

1. Build and install [openssl](https://github.com/openssl/openssl) for your target platform.
All openssl static builds used by vsd are available on [google drive](https://drive.google.com/drive/folders/11DaFm8pWwQoGpgbEbL8DmHce9ozTjWqz).

2. Install [Rust](https://www.rust-lang.org)

3. Download or clone repository.

```bash
git clone https://github.com/clitic/vsd.git
```

4. Install any C++ compiler and run cargo build command inside vsd directory.

## Windows

```powershell
$env:x86_64_PC_WINDOWS_MSVC_OPENSSL_DIR="C:\openssl-3.0.5-VC-WIN64A-static"
$env:x86_64_PC_WINDOWS_MSVC_OPENSSL_STATIC=$true
$env:x86_64_PC_WINDOWS_MSVC_NO_VENDOR=$true
cargo build --release
```

## Linux / MacOS

```bash
cargo build --release
```

## Linux with MUSL (On Linux 64-bit)

1. Build musl cross toolcain [musl-cross-make](https://github.com/richfelker/musl-cross-make) with C++ support.

```bash
$ curl -L https://github.com/richfelker/musl-cross-make/archive/refs/tags/v0.9.9.tar.gz | tar xzf -C .
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
$ PATH=musl-cross-make-0.9.9/output/bin:$PATH \
    CC=x86_64-linux-musl-gcc \
    CXX=x86_64-linux-musl-g++ \
    AR=x86_64-linux-musl-ar \
    x86_64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR=/content/openssl \
    x86_64_UNKNOWN_LINUX_MUSL_OPENSSL_NO_VENDOR=true \
    x86_64_UNKNOWN_LINUX_MUSL_OPENSSL_STATIC=true \
    cargo build --release --target x86_64-unknown-linux-musl
```

4. Check that binary is linking to any shared library or not.

```
$ ./musl-cross-make-0.9.9/output/bin/x86_64-linux-musl-readelf target/x86_64-unknown-linux-musl/release/vsd --dynamic
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
$ PATH=android-ndk-r22b/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
    AARCH64_LINUX_ANDROID_OPENSSL_DIR=openssl-v3.0.5-static-aarch64-linux-android30 \
    AARCH64_LINUX_ANDROID_OPENSSL_NO_VENDOR=true \
    AARCH64_LINUX_ANDROID_OPENSSL_STATIC=true \
    RUSTFLAGS="-C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib -C link-args=-Wl,--enable-new-dtags" \
    cargo build --release --target aarch64-linux-android
```

## Android (On Termux)

```bash
~ $ pkg upgrade
~ $ pkg install git rust
~ $ git clone https://github.com/clitic/vsd
~ $ cd vsd
~/vsd $ OPENSSL_INCLUDE_DIR=$PREFIX/include/openssl \
          OPENSSL_LIB_DIR=$PREFIX/lib \
          OPENSSL_NO_VENDOR=true \
          AR=llvm-ar \
          cargo build --release
```
