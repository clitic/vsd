#!/bin/bash

PACKAGES_DIR="$HOME/vsd-packages"
RELEASE_DIR="/mnt/c/Users/apoor/Downloads"

ANDROID_NDK_VERSION="r27d" # https://developer.android.com/ndk/downloads
MACOS_SDK_VERSION="26.1" # https://github.com/joseluisq/macosx-sdks/releases
PROTOC_VERSION="33.5" # https://github.com/protocolbuffers/protobuf/releases
VSD_VERSION="0.5.0" # vsd/Cargo.toml
ZIG_VERSION="0.15.2" # https://ziglang.org/download

. "$HOME/.cargo/env"

export PATH=$PACKAGES_DIR/protoc-$PROTOC_VERSION/bin:$PATH 
export PATH=$PACKAGES_DIR/zig-x86_64-linux-$ZIG_VERSION:$PATH 
export SDKROOT=$PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk

# Android

echo "Building aarch64-linux-android"
RUSTFLAGS="-C linker=aarch64-linux-android25-clang -C link-args=-Wl,-rpath=/data/data/com.termux/files/usr/lib" \
  PATH="$PACKAGES_DIR/android-ndk-$ANDROID_NDK_VERSION/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH" \
  AR="llvm-ar" \
  CC="aarch64-linux-android25-clang" \
  CXX="aarch64-linux-android25-clang++" \
  cargo build -p vsd --release --target aarch64-linux-android --no-default-features --features "license,native-tls-vendored"

echo "Packaging aarch64-linux-android"
cd target/aarch64-linux-android/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-aarch64-linux-android.tar.xz ./vsd
cd ../../../

# Darwin

echo "Building aarch64-apple-darwin"
RUSTFLAGS="-C linker=clang -C link-arg=--target=aarch64-apple-darwin -C link-arg=-isysroot -C link-arg=$PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk -C link-arg=-fuse-ld=lld" \
  AR="llvm-ar" \
  CC="clang --target=aarch64-apple-darwin -isysroot $PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk -fuse-ld=lld" \
  CXX="clang++ --target=aarch64-apple-darwin -isysroot $PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk -fuse-ld=lld" \
  cargo build -p vsd --release --target aarch64-apple-darwin

echo "Packaging aarch64-apple-darwin"
cd target/aarch64-apple-darwin/release
llvm-readobj vsd --needed-libs
tar -cJf $RELEASE_DIR/vsd-$VSD_VERSION-aarch64-apple-darwin.tar.xz ./vsd
cd ../../../

echo "Building x86_64-apple-darwin"
RUSTFLAGS="-C linker=clang -C link-arg=--target=x86_64-apple-darwin -C link-arg=-isysroot -C link-arg=$PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk -C link-arg=-fuse-ld=lld" \
  AR="llvm-ar" \
  CC="clang --target=x86_64-apple-darwin -isysroot $PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk -fuse-ld=lld" \
  CXX="clang++ --target=x86_64-apple-darwin -isysroot $PACKAGES_DIR/MacOSX$MACOS_SDK_VERSION.sdk -fuse-ld=lld" \
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
