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

## Android (On Linux 64-bit)

1. Install [NDK](https://developer.android.com/ndk/downloads)

```bash
$ wget https://dl.google.com/android/repository/android-ndk-r22b-linux-x86_64.zip
$ unzip android-ndk-r22b-linux-x86_64.zip
$ rm android-ndk-r22b-linux-x86_64.zip
```

2. Add android target aarch64-linux-android.

```bash
$ rustup target add aarch64-linux-android
$ printf '\n[target.aarch64-linux-android]\nlinker = "aarch64-linux-android30-clang"\n' >> $HOME/.cargo/config.toml
```

3. Now compile with target aarch64-linux-android. RUSTFLAGS can be removed if you do not want to build for termux.

```bash
$ PATH=android-ndk-r22b/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
    AARCH64_LINUX_ANDROID_OPENSSL_DIR=openssl-v3.0.5-static-aarch64-linux-android30 \
    AARCH64_LINUX_ANDROID_OPENSSL_STATIC=true \
    AARCH64_LINUX_ANDROID_OPENSSL_NO_VENDOR=true \
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

<!-- 
### x86_64-unknown-linux-musl (On Linux 64-bit)


```
# MUSL

# !apt install musl musl-dev musl-tools
!wget https://github.com/richfelker/musl-cross-make/archive/refs/tags/v0.9.9.tar.gz
!tar -xzf v0.9.9.tar.gz -C .
!rm v0.9.9.tar.gz

!cd musl-cross-make-0.9.9 && TARGET=x86_64-linux-musl make install
!cd musl-cross-make-0.9.9/output && tar -czf /content/musl-cross-make-v0.9.9-linux-64bit.tar.gz *
!rm -rf musl-cross-make-0.9.9
```

```
# openssl (MUSL)

# !apt install musl musl-dev musl-tools
!wget https://github.com/openssl/openssl/archive/refs/tags/openssl-3.0.5.tar.gz
!tar -xzf openssl-3.0.5.tar.gz -C .
!rm openssl-3.0.5.tar.gz

!cd openssl-openssl-3.0.5 && \
	CC=/content/musl-cross-make-v0.9.9/bin/x86_64-linux-musl-gcc \
	perl Configure linux-x86_64 no-shared --prefix=/content/openssl-build && \
  make && make install_sw

!cd openssl-build && tar -czf /content/openssl-v3.0.5-x86_64-linux-musl-static.tar.gz *
!rm -rf openssl-openssl-3.0.5 openssl-build
```

```
# openssl (Android 11+)

!wget https://github.com/openssl/openssl/archive/refs/tags/openssl-3.0.5.tar.gz
!tar -xzf openssl-3.0.5.tar.gz -C .
!rm openssl-3.0.5.tar.gz

cd openssl-openssl-3.0.5 && \
	ANDROID_NDK_ROOT=/content/android-ndk-r25 && \
	PATH=$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin:$ANDROID_NDK_ROOT/toolchains/arm-linux-androideabi-4.9/prebuilt/linux-x86_64/bin:$PATH && \
	perl Configure android-arm64 no-shared --prefix=/content/openssl-build --openssldir=/content/openssl-build -D__ANDROID_API__=30 && \
	make && make install_sw

!cd openssl-build && tar -czf /content/openssl-v3.0.5-android-arm64-android30-static.tar.gz *
!rm -rf openssl-openssl-3.0.5 openssl-build
```

# MUSL (Prebuilt)
!mkdir musl-cross-make-v0.9.9
!tar -xzf /content/drive/MyDrive/musl-cross-make-v0.9.9-linux-64bit.tar.gz -C musl-cross-make-v0.9.9

# openssl (Prebuilt)
!mkdir openssl-v3.0.5
!tar -xzf /content/drive/MyDrive/openssl-v3.0.5-x86_64-linux-musl-static.tar.gz -C openssl-v3.0.5

3. Add build target x86_64-unknown-linux-musl.

```bash
$ rustup target add x86_64-unknown-linux-musl
$ printf '\n[target.x86_64-unknown-linux-musl]\nlinker = "x86_64-linux-musl-gcc"\n' >> ~/.cargo/config.toml
```

```bash
$ PATH=musl-cross-make-v0.9.9/bin:$PATH \
    CC=x86_64-linux-musl-gcc \
    CXX=x86_64-linux-musl-g++ \
    PKG_CONFIG_ALLOW_CROSS=1 \
    OPENSSL_DIR=openssl-v3.0.5 \
    OPENSSL_STATIC=true \
    OPENSSL_NO_VENDOR=true \
    cargo build --release --target x86_64-unknown-linux-musl
```

!cd ./vsd/target/x86_64-unknown-linux-musl/release && tar -czf /content/vsd-v{version}-x86_64-unknown-linux-musl.tar.gz ./vsd -->

<!-- [openssl-v3.0.5-static-x86_64-linux-gnu.tar.gz](https://drive.google.com/file/d/1u7I6hNJ3P7Z6mzIQEY3VxiClJ99JbDm5/view?usp=sharing)
[openssl-v3.0.5-static-x86_64-linux-musl.tar.gz](https://drive.google.com/file/d/1V8qqgOl1fHgd2KLNplxsHgvwyvu67ITx/view?usp=sharing) -->
