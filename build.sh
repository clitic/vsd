#!/bin/bash

PACKAGES_DIR="$HOME/vsd-packages"
RELEASE_DIR="/mnt/c/Users/apoor/Downloads"

ANDROID_NDK_VERSION="r27c" # https://developer.android.com/ndk/downloads
MACOS_SDK_VERSION="15.4" # https://github.com/joseluisq/macosx-sdks/releases
PROTOC_VERSION="31.1" # https://github.com/protocolbuffers/protobuf/releases
VSD_VERSION="0.5.0" # vsd/Cargo.toml
ZIG_VERSION="0.14.1" # https://ziglang.org/download

. "$HOME/.cargo/env"
export PATH=$PACKAGES_DIR/protoc-$PROTOC_VERSION/bin:$PATH 
export PATH=$PACKAGES_DIR/zig-x86_64-linux-$ZIG_VERSION:$PATH 

# Android

echo "Building aarch64-linux-android"
PATH=$PACKAGES_DIR/android-ndk-$ANDROID_NDK_VERSION/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
  AR=llvm-ar \
  CC=aarch64-linux-android25-clang \
  CXX=aarch64-linux-android25-clang++ \
  RUSTFLAGS="-C linker=aarch64-linux-android25-clang -C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib" \
  cargo build -p vsd --release --target aarch64-linux-android --no-default-features --features "rustls"

echo "Packaging aarch64-linux-android"
cd target/aarch64-linux-android/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-aarch64-linux-android.tar.xz ./vsd
cd ../../../

# Darwin

echo "Building aarch64-apple-darwin"
PATH=$PACKAGES_DIR/osxcross/target/bin:$PATH \
  AR=aarch64-apple-darwin24.4-ar \
  CC=aarch64-apple-darwin24.4-clang \
  CXX=aarch64-apple-darwin24.4-clang++ \
  RUSTFLAGS="-C linker=aarch64-apple-darwin24.4-clang" \
  CRATE_CC_NO_DEFAULTS=true \
  cargo build -p vsd --release --target aarch64-apple-darwin

echo "Packaging aarch64-apple-darwin"
cd target/aarch64-apple-darwin/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-aarch64-apple-darwin.tar.xz ./vsd
cd ../../../

echo "Building x86_64-apple-darwin"
PATH=$PACKAGES_DIR/osxcross/target/bin:$PATH \
  AR=x86_64-apple-darwin24.4-ar \
  CC=x86_64-apple-darwin24.4-clang \
  CXX=x86_64-apple-darwin24.4-clang++ \
  RUSTFLAGS="-C linker=x86_64-apple-darwin24.4-clang" \
  CRATE_CC_NO_DEFAULTS=true \
  cargo build -p vsd --release --target x86_64-apple-darwin

echo "Packaging x86_64-apple-darwin"
cd target/x86_64-apple-darwin/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-x86_64-apple-darwin.tar.xz ./vsd
cd ../../../

# Linux

echo "Building aarch64-unknown-linux-musl"
cargo zigbuild -p vsd --release --target aarch64-unknown-linux-musl

echo "Packaging aarch64-unknown-linux-musl"
cd target/aarch64-unknown-linux-musl/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-aarch64-unknown-linux-musl.tar.xz ./vsd
cd ../../../

echo "Building x86_64-unknown-linux-musl"
cargo zigbuild -p vsd --release --target x86_64-unknown-linux-musl

echo "Packaging x86_64-unknown-linux-musl"
cd target/x86_64-unknown-linux-musl/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-x86_64-unknown-linux-musl.tar.xz ./vsd
cd ../../../

# Windows

echo "Building aarch64-pc-windows-msvc"
cargo xwin build -p vsd --release --target aarch64-pc-windows-msvc

echo "Packaging aarch64-pc-windows-msvc"
cd target/aarch64-pc-windows-msvc/release
llvm-readobj vsd.exe --needed-libs
zip $RELEASE_DIR/vsd-$VSD_VERSION-aarch64-pc-windows-msvc.zip ./vsd.exe
cd ../../../

echo "Building x86_64-pc-windows-msvc"
cargo xwin build -p vsd --release --target x86_64-pc-windows-msvc

echo "Packaging x86_64-pc-windows-msvc"
cd target/x86_64-pc-windows-msvc/release
llvm-readobj vsd.exe --needed-libs
zip $RELEASE_DIR/vsd-$VSD_VERSION-x86_64-pc-windows-msvc.zip ./vsd.exe
cd ../../../
