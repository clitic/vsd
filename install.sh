#!/bin/bash

PACKAGES_DIR="$HOME/vsd-packages"

ANDROID_NDK_VERSION="r27d" # https://developer.android.com/ndk/downloads
MACOS_SDK_VERSION="26.1" # https://github.com/joseluisq/macosx-sdks/releases
PROTOC_VERSION="33.5" # https://github.com/protocolbuffers/protobuf/releases
ZIG_VERSION="0.15.2" # https://ziglang.org/download

echo "Installing Build Dependencies"
sudo apt update
sudo apt upgrade -y
sudo apt install -y clang llvm perl zip unzip

rm -rf $PACKAGES_DIR
mkdir -p $PACKAGES_DIR

echo "Installing Android NDK v$ANDROID_NDK_VERSION"
curl -L https://dl.google.com/android/repository/android-ndk-$ANDROID_NDK_VERSION-linux.zip -o android-ndk-$ANDROID_NDK_VERSION-linux.zip
unzip android-ndk-$ANDROID_NDK_VERSION-linux.zip -d $PACKAGES_DIR
rm android-ndk-$ANDROID_NDK_VERSION-linux.zip

echo "Installing MacOSX SDK v$MACOS_SDK_VERSION"
curl -L https://github.com/joseluisq/macosx-sdks/releases/download/$MACOS_SDK_VERSION/MacOSX$MACOS_SDK_VERSION.sdk.tar.xz | tar xJC $PACKAGES_DIR

echo "Installing Protoc v$PROTOC_VERSION"
curl -L https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/protoc-$PROTOC_VERSION-linux-x86_64.zip -o protoc-$PROTOC_VERSION-linux-x86_64.zip
unzip protoc-$PROTOC_VERSION-linux-x86_64.zip -d $PACKAGES_DIR/protoc-$PROTOC_VERSION
rm protoc-$PROTOC_VERSION-linux-x86_64.zip

echo "Installing Zig v$ZIG_VERSION"
curl -L https://ziglang.org/download/$ZIG_VERSION/zig-x86_64-linux-$ZIG_VERSION.tar.xz | tar xJC $PACKAGES_DIR

echo "Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

. "$HOME/.cargo/env"

rustup target add \
  aarch64-apple-darwin \
  aarch64-linux-android \
  aarch64-pc-windows-msvc \
  aarch64-unknown-linux-musl \
  x86_64-apple-darwin \
  x86_64-pc-windows-msvc \
  x86_64-unknown-linux-musl

echo "Installing cargo-zigbuild and cargo-xwin"
cargo install cargo-zigbuild cargo-xwin
