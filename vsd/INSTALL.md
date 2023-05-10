# INSTALL.md

## Through Cargo

```bash
cargo install vsd
```

## Linux (x86_64)

```bash
curl -L https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-x86_64-unknown-linux-musl.tar.gz | tar xz -C /usr/local/bin
```

## MacOS 12.3+ (x86_64)

```bash
curl -L https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-x86_64-apple-darwin.tar.gz | tar xz -C /usr/local/bin
```

## Android 7+ (Termux) (aarch64)

```bash
curl -L https://github.com/clitic/vsd/releases/download/v0.2.5/vsd-v0.2.5-aarch64-linux-android.tar.gz | tar xz -C $PREFIX/bin
```

You can also build vsd lower android versions see [steps](https://github.com/clitic/vsd/blob/main/BUILD.md#android-on-termux).
Also, see [running on android](https://github.com/clitic/vsd/blob/main/docs/running-on-android.md).
