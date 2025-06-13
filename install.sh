#!/bin/bash

PACKAGES_DIR="$HOME/vsd-packages"

ANDROID_NDK_VERSION="r27c" # https://developer.android.com/ndk/downloads
MACOS_SDK_VERSION="15.4" # https://github.com/joseluisq/macosx-sdks/releases
PROTOC_VERSION="31.1" # https://github.com/protocolbuffers/protobuf/releases
ZIG_VERSION="0.14.1" # https://ziglang.org/download

echo "Installing Build Dependencies"
sudo apt update
sudo apt upgrade -y
sudo apt install -y zip unzip # required by script
sudo apt install -y build-essential libssl-dev pkgconf # required by vsd
sudo apt install -y bzip2 clang cmake cpio git libssl-dev libxml2-dev llvm-dev lzma-dev patch python3 uuid-dev zlib1g-dev xz-utils # required by osxcross

rm -rf $PACKAGES_DIR
mkdir -p $PACKAGES_DIR

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

echo "Installing cargo-zigbuild"
cargo install cargo-zigbuild

echo "Installing cargo-xwin"
cargo install cargo-xwin

echo "Installing Protoc v$PROTOC_VERSION"
curl -L https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/protoc-$PROTOC_VERSION-linux-x86_64.zip -o protoc-$PROTOC_VERSION-linux-x86_64.zip
unzip protoc-$PROTOC_VERSION-linux-x86_64.zip -d $PACKAGES_DIR/protoc-$PROTOC_VERSION
rm protoc-$PROTOC_VERSION-linux-x86_64.zip

echo "Installing Zig v$ZIG_VERSION"
curl -L https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz | tar xJC $PACKAGES_DIR

echo "Installing Android NDK v$ANDROID_NDK_VERSION"
curl -L https://dl.google.com/android/repository/android-ndk-$ANDROID_NDK_VERSION-linux.zip -o android-ndk-$ANDROID_NDK_VERSION-linux.zip
unzip android-ndk-$ANDROID_NDK_VERSION-linux.zip -d $PACKAGES_DIR
rm android-ndk-$ANDROID_NDK_VERSION-linux.zip

echo "Installing Osxcross"
git clone https://github.com/tpoechtrager/osxcross $PACKAGES_DIR/osxcross
curl -L https://github.com/joseluisq/macosx-sdks/releases/download/$MACOS_SDK_VERSION/MacOSX$MACOS_SDK_VERSION.sdk.tar.xz -o $PACKAGES_DIR/osxcross/tarballs/MacOSX$MACOS_SDK_VERSION.sdk.tar.xz
cd $PACKAGES_DIR/osxcross
UNATTENDED=1 ./build.sh
